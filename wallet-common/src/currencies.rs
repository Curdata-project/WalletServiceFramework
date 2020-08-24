use crate::query::QueryParam;
use common_structure::digital_currency::DigitalCurrencyWrapper;
use common_structure::transaction::TransactionWrapper;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CurrencyStatus {
    /// 可用货币
    Avail,
    /// 等待确认的货币
    WaitConfirm,
    /// 交易锁定货币
    Lock,
}

impl CurrencyStatus {
    pub fn to_int(self) -> i16 {
        match self {
            CurrencyStatus::Avail => 0,
            CurrencyStatus::WaitConfirm => 1,
            CurrencyStatus::Lock => 2,
        }
    }
}

impl From<i16> for CurrencyStatus {
    fn from(status: i16) -> Self {
        match status {
            0 => CurrencyStatus::Avail,
            1 => CurrencyStatus::WaitConfirm,
            2 => CurrencyStatus::Lock,
            _ => CurrencyStatus::Avail,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CurrencyEntity {
    AvailEntity {
        id: String,
        owner_uid: String,
        value: u64,
        currency: DigitalCurrencyWrapper,
        currency_str: String,
        txid: String,
        update_time: i64,
        last_owner_id: String,
    },
    LockEntity {
        id: String,
        owner_uid: String,
        value: u64,
        currency: DigitalCurrencyWrapper,
        currency_str: String,
        txid: String,
        update_time: i64,
        last_owner_id: String,
    },
    WaitConfirmEntity {
        id: String,
        owner_uid: String,
        value: u64,
        transaction: TransactionWrapper,
        transaction_str: String,
        txid: String,
        update_time: i64,
        last_owner_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AddCurrencyParam {
    AvailEntity {
        owner_uid: String,
        currency_str: String,
        txid: String,
        last_owner_id: String,
    },
    WaitConfirmEntity {
        owner_uid: String,
        transaction_str: String,
        txid: String,
        last_owner_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockCurrencyParam {
    pub owner_uid: String,
    pub currency: DigitalCurrencyWrapper,
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
    pub has_wait_confirm: bool,
    pub owner_uid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyStatisticsItem {
    pub value: u64,
    pub num: u64,
}

impl PartialEq for CurrencyStatisticsItem {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl PartialOrd for CurrencyStatisticsItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.value.partial_cmp(&self.value)
    }
}

impl Eq for CurrencyStatisticsItem {}

impl Ord for CurrencyStatisticsItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickSpecifiedNumCurrencyParam {
    pub items: Vec<CurrencyStatisticsItem>,
    pub owner_uid: String,
}
