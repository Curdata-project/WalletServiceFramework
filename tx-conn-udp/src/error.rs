use ewf_core::error::Error as EwfError;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    TXConnectError,
    TXConnectBroken,
    TXConnectUrlUnvalid,
    TXConnectCollision,
    TXBindError,
    TXRouteInfoNotFound,
}

impl Error {
    pub fn to_ewf_error(self) -> EwfError {
        match self {
            Error::TXConnectError => EwfError::OtherError("连接失败".to_string()),
            Error::TXConnectBroken => EwfError::OtherError("交易中断".to_string()),
            Error::TXConnectUrlUnvalid => EwfError::OtherError("交易端口格式错误".to_string()),
            Error::TXConnectCollision => EwfError::OtherError("交易ID已使用".to_string()),
            Error::TXBindError => EwfError::OtherError("无法绑定本地端口".to_string()),
            Error::TXRouteInfoNotFound => EwfError::OtherError("找不到对应的路由信息".to_string()),
        }
    }
}
