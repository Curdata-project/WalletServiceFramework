use common_structure::digital_currency::DigitalCurrencyWrapper;
use common_structure::transaction::TransactionWrapper;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CurrencyStatus {
    Avail,
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
pub enum CurrencyEntity {
    AvailEntity {
        id: String,
        currency: DigitalCurrencyWrapper,
        txid: String,
        update_time: i64,
        last_owner_id: String,
    },
    LockEntity {
        id: String,
        transaction: TransactionWrapper,
        txid: String,
        update_time: i64,
        last_owner_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AddCurrencyParam {
    AvailEntity {
        currency: DigitalCurrencyWrapper,
        txid: String,
        last_owner_id: String,
    },
    LockEntity {
        transaction: TransactionWrapper,
        txid: String,
        last_owner_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticsItem {
    pub value: u64,
    pub num: u64,
}

impl PartialEq for StatisticsItem {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl PartialOrd for StatisticsItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.value.partial_cmp(&self.value)
    }
}

impl Eq for StatisticsItem {}

impl Ord for StatisticsItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockCurrencyParam {
    pub currency: DigitalCurrencyWrapper,
}
