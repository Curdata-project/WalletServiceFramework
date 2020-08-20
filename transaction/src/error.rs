use ewf_core::error::Error as EwfError;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    TXMsgPackBroken,
    // 状态机已销毁
    TXMachineDestoryed,
    TXPaymentPlanNotForUser,
    TXCLOCKSKEWTOOLARGE,
    TXSequenceNotExpect,
    TXPayBalanceNotEnough,

    TXExpectError,
}

impl Error {
    pub fn to_ewf_error(self) -> EwfError {
        match self {
            Error::TXMsgPackBroken => EwfError::OtherError("交易异常, 已终止".to_string()),
            Error::TXMachineDestoryed => EwfError::OtherError("已终止的交易".to_string()),
            Error::TXPaymentPlanNotForUser => {
                EwfError::OtherError("用户不涉及此次交易".to_string())
            }
            Error::TXCLOCKSKEWTOOLARGE => EwfError::OtherError("交易时钟偏差过大".to_string()),
            Error::TXSequenceNotExpect => EwfError::OtherError("交易序列异常".to_string()),
            Error::TXPayBalanceNotEnough => EwfError::OtherError("交易金额不足".to_string()),
            Error::TXExpectError => EwfError::OtherError("交易意外出错".to_string()),
        }
    }
}
