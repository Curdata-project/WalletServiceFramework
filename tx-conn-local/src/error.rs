use ewf_core::error::Error as EwfError;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    TXConnectError,
    TXConnectBroken,
}

impl Error {
    pub fn to_ewf_error(self) -> EwfError {
        match self {
            Error::TXConnectError => EwfError::OtherError("连接失败".to_string()),
            Error::TXConnectBroken => EwfError::OtherError("交易中断".to_string()),
        }
    }
}
