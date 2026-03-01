//! Notification and message handling for the LSP client.
//!
//! Contains methods for handling server notifications (especially
//! `publishDiagnostics`), draining pending messages, reading notifications
//! with timeouts, and non-blocking message reading.

use serde_json::Value;
use std::{
    io::{BufRead, ErrorKind},
    sync::mpsc,
    thread,
    time::Duration,
};
use vize_carton::String;

use super::TsgoLspClient;

impl TsgoLspClient {
    /// Drain any pending messages without blocking
    pub(crate) fn drain_pending_messages(&mut self) {
        while let Some(Ok(msg)) = self.try_read_message_nonblocking() {
            self.handle_notification(&msg);
        }
    }

    /// Wait for diagnostics to be published for all opened files
    /// Waits until we receive diagnostics for expected_count files, or idle timeout
    pub fn wait_for_diagnostics(&mut self, expected_count: usize) {
        use std::time::Instant;

        let max_wait = Duration::from_secs(30); // Maximum total wait
        let idle_timeout = Duration::from_millis(30); // Reduced idle timeout (was 200ms)
        let start = Instant::now();
        let mut last_message: Option<Instant> = None;
        let initial_diag_count = self.diagnostics.len();

        // Read messages until we have enough diagnostics, idle timeout, or max wait
        loop {
            // Check for max wait timeout
            if start.elapsed() > max_wait {
                break;
            }

            // Check if we have diagnostics for all expected files
            let new_diags = self.diagnostics.len() - initial_diag_count;
            if new_diags >= expected_count {
                // Got all diagnostics, wait just a tiny bit more for any stragglers
                thread::sleep(Duration::from_millis(5));
                self.drain_pending_messages();
                break;
            }

            // Check for idle timeout (only after receiving at least one message)
            if let Some(last) = last_message {
                if last.elapsed() > idle_timeout {
                    break;
                }
            }

            // Try reading a message
            match self.try_read_message_nonblocking() {
                Some(Ok(msg)) => {
                    last_message = Some(Instant::now()); // Reset idle timer
                    self.handle_notification(&msg);
                }
                Some(Err(_)) => break,
                None => {
                    // No data available, wait a bit
                    thread::sleep(Duration::from_millis(1));
                }
            }
        }
    }

    /// Read notifications with timeout using a background thread
    pub(crate) fn read_notifications(&mut self) -> Result<(), String> {
        // Create channel for timeout
        let (tx, rx) = mpsc::channel();

        // Spawn a thread to signal after timeout (50ms for fast response)
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            let _ = tx.send(());
        });

        // Try to read messages until we get diagnostics or timeout
        loop {
            // Check for timeout
            if rx.try_recv().is_ok() {
                break;
            }

            // Try reading a message
            match self.try_read_message_nonblocking() {
                Some(Ok(msg)) => {
                    let method = msg.get("method").and_then(|m| m.as_str());
                    self.handle_notification(&msg);
                    // If we got diagnostics, we can stop
                    if method == Some("textDocument/publishDiagnostics") {
                        break;
                    }
                }
                Some(Err(_)) => break,
                None => {
                    // No data available, wait a bit
                    thread::sleep(Duration::from_millis(1));
                }
            }
        }

        Ok(())
    }

    /// Try to read a message without blocking forever
    pub(crate) fn try_read_message_nonblocking(&mut self) -> Option<Result<Value, String>> {
        // Check if there's data available using fill_buf
        // With non-blocking mode, fill_buf returns WouldBlock if no data
        match self.stdout.fill_buf() {
            Ok([]) => None,                                      // EOF
            Ok(_) => Some(self.read_message()),                  // Data available
            Err(e) if e.kind() == ErrorKind::WouldBlock => None, // No data yet
            Err(_) => None,                                      // Other error
        }
    }

    /// Handle a notification or request message
    pub(crate) fn handle_notification(&mut self, msg: &Value) {
        if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
            // Check if this is a request (has id) that needs a response
            if let Some(id) = msg.get("id") {
                // This is a request from the server, send an empty response
                let response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": null
                });
                let _ = self.send_message(&response);
                return;
            }

            // Handle notification
            if method == "textDocument/publishDiagnostics" {
                if let Some(params) = msg.get("params") {
                    if let (Some(uri), Some(diagnostics)) =
                        (params.get("uri"), params.get("diagnostics"))
                    {
                        if let (Some(uri_str), Some(diag_array)) =
                            (uri.as_str(), diagnostics.as_array())
                        {
                            let diags: Vec<super::LspDiagnostic> = diag_array
                                .iter()
                                .filter_map(|d| serde_json::from_value(d.clone()).ok())
                                .collect();
                            self.diagnostics.insert(uri_str.into(), diags);
                        }
                    }
                }
            }
        }
    }
}
