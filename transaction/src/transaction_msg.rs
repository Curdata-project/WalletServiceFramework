use crate::error::Error;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use wallet_common::currencies::CurrencyStatisticsItem;
use wallet_common::transaction::TransactionExchangerItem;

pub fn get_msgtype(pack: &Value) -> String {
    if pack["txmsgtype"] != Value::Null {
        return pack["txmsgtype"].as_str().unwrap().to_string();
    }
    "".to_string()
}

pub trait TXMsgPackageData: Sized + Serialize + for<'de> Deserialize<'de> {
    fn to_msgpack(self) -> Value {
        json!({
            "txmsgtype": Self::get_msgtype(),
            "data": self,
        })
    }

    fn from_msgpack(pack: Value) -> Result<Self, Error> {
        match serde_json::from_value(pack["data"].clone()) {
            Ok(ans) => Ok(ans),
            Err(_) => Err(Error::TXMsgPackBroken),
        }
    }

    fn get_msgtype() -> String;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionContextSyn {
    pub timestamp: i64,
    pub exchangers: Vec<TransactionExchangerItem>,
}

impl TXMsgPackageData for TransactionContextSyn {
    fn get_msgtype() -> String {
        "TransactionContextSyn".to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionContextAck {}

impl TXMsgPackageData for TransactionContextAck {
    fn get_msgtype() -> String {
        "TransactionContextAck".to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyStat {
    pub statistics: Vec<CurrencyStatisticsItem>,
}

impl TXMsgPackageData for CurrencyStat {
    fn get_msgtype() -> String {
        "CurrencyStat".to_string()
    }
}
