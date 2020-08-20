use common_structure::digital_currency::DigitalCurrencyWrapper;
use common_structure::transaction::TransactionWrapper;
use serde::{Deserialize, Serialize};

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
pub struct UnlockCurrencyParam {
    pub currency: DigitalCurrencyWrapper,
}
