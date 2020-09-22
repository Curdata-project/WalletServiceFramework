use crate::query::QueryParam;
use common_structure::digital_currency::DigitalCurrencyWrapper;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CurrencyStatus {
    /// 可用货币
    Avail,
    /// 交易锁定货币
    Lock,
}

impl CurrencyStatus {
    pub fn to_int(self) -> i16 {
        match self {
            CurrencyStatus::Avail => 0,
            CurrencyStatus::Lock => 1,
        }
    }
}

impl From<i16> for CurrencyStatus {
    fn from(status: i16) -> Self {
        match status {
            0 => CurrencyStatus::Avail,
            1 => CurrencyStatus::Lock,
            _ => CurrencyStatus::Avail,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyEntity {
    pub id: String,
    pub owner_uid: String,
    pub amount: u64,
    pub currency: DigitalCurrencyWrapper,
    pub currency_str: String,
    pub txid: String,
    pub update_time: i64,
    pub last_owner_id: String,
    pub status: CurrencyStatus,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyEntityShort {
    pub id: String,
    pub amount: u64,
    pub status: CurrencyStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddCurrencyParam {
    pub owner_uid: String,
    pub currency_str: String,
    pub txid: String,
    pub last_owner_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmCurrencyParam {
    pub owner_uid: String,
    pub currency_str: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyQuery {
    pub query_param: QueryParam,
    pub uid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCurrencyStatisticsParam {
    pub has_avail: bool,
    pub has_lock: bool,
    pub owner_uid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnLockCurrencyParam {
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyDepositParam {
    pub uid: String,
    pub bank_num: String,
    pub amount: u64,
    pub currencys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyWithdrawParam {
    pub uid: String,
    pub bank_num: String,
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyWithdrawResult {
    pub currencys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyConvertInfo {
    pub uid: String,
    pub amount: u64,
    pub plan: Vec<(u64, u64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyConvertParam {
    pub url: String,
    pub timeout: u64,
    pub info: CurrencyConvertInfo,
}
