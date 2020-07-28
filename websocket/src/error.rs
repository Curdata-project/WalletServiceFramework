use serde::{Serialize};
use jsonrpc_lite::Error as JsonRpcError;


#[derive(Serialize)]
pub enum WalletError {
    // websock 错误
    ParamIsNone,
}

impl Into<JsonRpcError> for WalletError {
    fn into(self) -> JsonRpcError {
        let (code, message) = match self {
            WalletError::ParamIsNone => (1000i64, "Param is none"),
        };

        JsonRpcError {
            code,
            message: message.to_string(),
            data: None,
        }
    }
}
