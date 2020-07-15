use jsonrpc_lite::{Error as JsonRpcError};
use serde::{Deserialize, Serialize};
use std::convert::Into;

#[derive(Serialize, Deserialize)]
pub enum WallerError {
    // websock 错误
    WebSockServerBindError,
    WebSockServerAcceptConnError,
    WebSockServerGetPeerError,

    ModuleBusNotFound,
    

    DatabaseOpenError,
}

#[derive(Serialize)]
pub enum StateMachineError {
    StateMachineReset,
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
