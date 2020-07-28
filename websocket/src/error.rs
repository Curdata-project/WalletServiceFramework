use serde::{Serialize};
use jsonrpc_lite::Error as JsonRpcError;
use actix::prelude::*;
use ewf_core::error::Error as EwfError;


#[derive(Serialize)]
pub enum WalletError {
    // 模块调用错误，一般是actor投递失败
    ServerModuleCallError,
    ModuleParamTypeError,

    // 其他意外错误，详细记录error级别日志并抛出ExpectError
    ExpectError,
}

impl Into<JsonRpcError> for WalletError {
    fn into(self) -> JsonRpcError {
        let (code, message) = match self {
            WalletError::ServerModuleCallError => (1000i64, "Server module call error"),
            WalletError::ModuleParamTypeError => (1001i64, "Server module call use incorrect param type"),
            WalletError::ExpectError => (1001i64, "Server module call use incorrect param type"),
        };

        JsonRpcError {
            code,
            message: message.to_string(),
            data: None,
        }
    }
}

impl From<EwfError> for WalletError
{
    fn from(e: EwfError) -> WalletError {
        WalletError::ServerModuleCallError
    }
}

impl From<MailboxError> for WalletError {
    fn from(e: MailboxError) -> WalletError {
        WalletError::ServerModuleCallError
    }
}