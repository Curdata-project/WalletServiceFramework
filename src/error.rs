use jsonrpc_lite::JsonRpc;
use jsonrpc_lite::{Error as JsonRpcError, ErrorCode as JsonRpcErrorCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::convert::Into;

#[derive(Serialize)]
pub enum WallerError {
    // websock 错误
    WebSockServerBindError,
    WebSockServerAcceptConnError,
    WebSockServerGetPeerError,
}

impl Into<JsonRpcError> for WallerError {
    fn into(self) -> JsonRpcError {
        JsonRpcError {
            code: 1000i64,
            message: "test".to_string(),
            data: None,
        }
    }
}
