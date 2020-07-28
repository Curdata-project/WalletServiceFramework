use serde::{Serialize};
use jsonrpc_lite::Error as JsonRpcError;
use ewf_core::error::Error as EwfError;
use actix::prelude::*;


#[derive(Serialize)]
pub enum WalletError {
    // 模块调用错误，一般是actor投递失败
    ServerModuleCallError(String),
    ModuleParamTypeError,

    // 其他意外错误，详细记录error级别日志并抛出ExpectError
    ExpectError(String),
}

impl Into<JsonRpcError> for WalletError {
    fn into(self) -> JsonRpcError {
        let (code, message) = match self {
            // WalletError::ServerModuleCallError(_) => (1000i64, "Server module call error"),
            // WalletError::ModuleParamTypeError => (1001i64, "Server module call use incorrect param type"),
            // WalletError::ExpectError(_) => (1001i64, "Server module call use incorrect param type"),
            WalletError::ServerModuleCallError(err) => (1000i64, format!("{:?}", err)),
            WalletError::ModuleParamTypeError => (1001i64, "Server module call use incorrect param type".to_string()),
            WalletError::ExpectError(err) => (1001i64, format!("{:?}", err)),
        };

        JsonRpcError {
            code,
            message: message,
            data: None,
        }
    }
}

impl From<EwfError> for WalletError
{
    fn from(e: EwfError) -> WalletError {
        WalletError::ServerModuleCallError(format!("{:?}", e))
    }
}

impl From<MailboxError> for WalletError {
    fn from(e: MailboxError) -> WalletError {
        WalletError::ServerModuleCallError(format!("{:?}", e))
    }
}
