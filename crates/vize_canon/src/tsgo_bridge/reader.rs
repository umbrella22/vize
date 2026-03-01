//! Response reader task for the tsgo bridge.
//!
//! Handles reading and dispatching LSP messages from the tsgo process stdout,
//! including responses to client requests, server-initiated requests, and
//! notifications like `publishDiagnostics`.

#[allow(clippy::disallowed_types)]
use std::sync::Arc;

use dashmap::DashMap;
use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::ChildStdout as TokioChildStdout,
    sync::oneshot,
};

use super::{
    protocol::JsonRpcMessage,
    types::{LspDiagnostic, TsgoBridgeError},
};
use vize_carton::cstr;
use vize_carton::String;

/// Type alias for pending requests map.
#[allow(clippy::disallowed_types)]
pub(crate) type PendingMap = Arc<DashMap<u64, oneshot::Sender<Result<Value, TsgoBridgeError>>>>;

/// Type alias for diagnostics cache map.
#[allow(clippy::disallowed_types)]
pub(crate) type DiagnosticsCache = Arc<DashMap<String, Vec<LspDiagnostic>>>;

/// Type alias for shared stdin writer.
#[allow(clippy::disallowed_types)]
pub(crate) type SharedStdin =
    Arc<tokio::sync::Mutex<Option<tokio::io::BufWriter<tokio::process::ChildStdin>>>>;

/// Type alias for open documents tracking (URI -> version).
#[allow(clippy::disallowed_types)]
pub(crate) type OpenDocuments = Arc<DashMap<String, i32>>;

/// Start the response reader task that processes messages from tsgo stdout.
#[allow(clippy::disallowed_types, clippy::disallowed_methods)]
pub(crate) fn start_reader_task(
    stdout: TokioChildStdout,
    pending: PendingMap,
    diagnostics_cache: DiagnosticsCache,
    stdin: SharedStdin,
) {
    tokio::spawn(async move {
        tracing::info!("tsgo_bridge: reader task started");
        let mut reader = BufReader::new(stdout);
        #[allow(clippy::disallowed_types)]
        let mut headers = std::string::String::new();
        let mut content_length: usize = 0;

        loop {
            headers.clear();
            tracing::debug!("tsgo_bridge: reader waiting for next message...");

            // Read headers
            loop {
                #[allow(clippy::disallowed_types)]
                let mut line = std::string::String::new();
                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        tracing::warn!("tsgo_bridge: reader EOF");
                        return;
                    }
                    Ok(n) => {
                        tracing::debug!("tsgo_bridge: read header line ({} bytes): {:?}", n, line);
                        if line == "\r\n" || line == "\n" {
                            break;
                        }
                        if line.to_lowercase().starts_with("content-length:") {
                            if let Some(len_str) = line.split(':').nth(1) {
                                content_length = len_str.trim().parse().unwrap_or(0);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("tsgo_bridge: reader error: {}", e);
                        return;
                    }
                }
            }

            if content_length == 0 {
                tracing::warn!("tsgo_bridge: content_length is 0, skipping");
                continue;
            }

            tracing::info!("tsgo_bridge: reading {} bytes", content_length);

            // Read content
            let mut content = vec![0u8; content_length];
            if reader.read_exact(&mut content).await.is_err() {
                tracing::error!("tsgo_bridge: failed to read content");
                continue;
            }

            // Log raw content for debugging
            let raw_str = String::from_utf8_lossy(&content);
            tracing::info!(
                "tsgo_bridge: raw message (first 300 chars): {}",
                &raw_str[..raw_str.len().min(300)]
            );

            // Parse message (response or notification)
            let message: JsonRpcMessage = match serde_json::from_slice(&content) {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("tsgo_bridge: failed to parse message: {}", e);
                    tracing::error!("tsgo_bridge: raw content: {}", raw_str);
                    continue;
                }
            };

            tracing::info!(
                "tsgo_bridge: received message id={:?} method={:?}",
                message.id,
                message.method
            );

            // Handle response (has id, no method) - this is a response to our request
            if let Some(ref id) = message.id {
                // Check if this is a server request (has both id and method)
                if message.method.is_some() {
                    // This is a request FROM the server TO the client
                    // We need to respond with an empty result (like CLI does)
                    tracing::info!(
                        "tsgo_bridge: server request received, method={:?}, sending empty response",
                        message.method
                    );

                    // Send empty response
                    let response = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": Value::Null
                    });
                    if let Ok(response_content) = serde_json::to_string(&response) {
                        let response_msg = cstr!(
                            "Content-Length: {}\r\n\r\n{response_content}",
                            response_content.len(),
                        )
                        .to_string();
                        let mut stdin_guard = stdin.lock().await;
                        if let Some(ref mut writer) = *stdin_guard {
                            let _ = writer.write_all(response_msg.as_bytes()).await;
                            let _ = writer.flush().await;
                            tracing::info!("tsgo_bridge: sent empty response for server request");
                        }
                    }
                } else if let Some(numeric_id) = id.as_u64() {
                    // This is a response to our request
                    if let Some((_, sender)) = pending.remove(&numeric_id) {
                        let result = if let Some(error) = message.error {
                            tracing::warn!(
                                "tsgo_bridge: error response: {} - {}",
                                error.code,
                                error.message
                            );
                            Err(TsgoBridgeError::ResponseError {
                                code: error.code,
                                message: error.message.into(),
                            })
                        } else {
                            Ok(message.result.unwrap_or(Value::Null))
                        };
                        let _ = sender.send(result);
                    }
                }
            }
            // Handle notification (no id, has method)
            else if let Some(ref method) = message.method {
                if method == "textDocument/publishDiagnostics" {
                    if let Some(ref params) = message.params {
                        if let (Some(uri), Some(diagnostics)) = (
                            params.get("uri").and_then(|v| v.as_str()),
                            params.get("diagnostics"),
                        ) {
                            if let Ok(diags) =
                                serde_json::from_value::<Vec<LspDiagnostic>>(diagnostics.clone())
                            {
                                tracing::info!(
                                    "tsgo_bridge: received {} diagnostics for {}",
                                    diags.len(),
                                    uri
                                );
                                diagnostics_cache.insert(uri.into(), diags);
                            }
                        }
                    }
                }
            }
        }
    });
}
