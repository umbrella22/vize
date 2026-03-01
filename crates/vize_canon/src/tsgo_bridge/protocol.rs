//! JSON-RPC protocol types for LSP communication.
//!
//! Internal types used for serializing/deserializing JSON-RPC messages
//! between the bridge and the tsgo process.
//!
//! Uses `std::string::String` for serde deserialization compatibility.
#![allow(clippy::disallowed_types)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use vize_carton::String;

/// JSON-RPC request.
#[derive(Debug, Serialize)]
pub(crate) struct JsonRpcRequest {
    pub(crate) jsonrpc: &'static str,
    pub(crate) id: u64,
    pub(crate) method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) params: Option<Value>,
}

/// JSON-RPC notification (no id).
#[derive(Debug, Serialize)]
pub(crate) struct JsonRpcNotification {
    pub(crate) jsonrpc: &'static str,
    pub(crate) method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) params: Option<Value>,
}

/// JSON-RPC ID can be number or string.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::disallowed_types)]
pub(crate) enum JsonRpcId {
    Number(u64),
    String(std::string::String),
}

impl JsonRpcId {
    pub(crate) fn as_u64(&self) -> Option<u64> {
        match self {
            JsonRpcId::Number(n) => Some(*n),
            JsonRpcId::String(s) => s.parse().ok(),
        }
    }
}

/// JSON-RPC message (response or notification).
#[derive(Debug, Deserialize)]
#[allow(dead_code, clippy::disallowed_types)]
pub(crate) struct JsonRpcMessage {
    pub(crate) jsonrpc: std::string::String,
    pub(crate) id: Option<JsonRpcId>,
    pub(crate) result: Option<Value>,
    pub(crate) error: Option<JsonRpcError>,
    /// Method name for notifications
    pub(crate) method: Option<std::string::String>,
    /// Params for notifications
    pub(crate) params: Option<Value>,
}

/// JSON-RPC error.
#[derive(Debug, Deserialize)]
#[allow(dead_code, clippy::disallowed_types)]
pub(crate) struct JsonRpcError {
    pub(crate) code: i64,
    pub(crate) message: std::string::String,
    pub(crate) data: Option<Value>,
}
