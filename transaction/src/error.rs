use actix::prelude::*;
use ewf_core::error::Error as EwfError;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Error {
    TXMsgPackBroken,
    // 状态机已销毁
    TXMachineDestoryed,
    TXPaymentPlanNotForUser,
    TXCLOCKSKEWTOOLARGE,
    TXSequenceNotExpect,
    TXPayBalanceNotEnough,
    TXPayNotAvailChangePlan,
    TXCurrencyPlanNotValid,

    TXExpectError,
}

impl Error {
    pub fn to_ewf_error(self) -> EwfError {
        EwfError::OtherError(self.to_string())
    }

    pub fn to_string(self) -> String {
        match self {
            Error::TXMsgPackBroken => "交易异常, 已终止".to_string(),
            Error::TXMachineDestoryed => "已终止的交易".to_string(),
            Error::TXPaymentPlanNotForUser => "用户不涉及此次交易".to_string(),
            Error::TXCLOCKSKEWTOOLARGE => "交易时钟偏差过大".to_string(),
            Error::TXSequenceNotExpect => "交易序列异常".to_string(),
            Error::TXPayBalanceNotEnough => "交易金额不足".to_string(),
            Error::TXPayNotAvailChangePlan => "没有可用找零方案".to_string(),
            Error::TXCurrencyPlanNotValid => "交易收到的支付方案不合法".to_string(),
            Error::TXExpectError => "交易意外出错".to_string(),
        }
    }
}

impl From<EwfError> for Error {
    fn from(e: EwfError) -> Error {
        log::warn!("found a probable problem {:?}", e);
        Error::TXExpectError
    }
}

impl From<MailboxError> for Error {
    fn from(e: MailboxError) -> Error {
        log::warn!("found a probable problem {:?}", e);
        Error::TXExpectError
    }
}
