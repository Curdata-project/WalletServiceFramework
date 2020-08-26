use crate::error::Error;
use crate::tx_payload_mgr::PeerCurrencyPlan;
use common_structure::transaction::TransactionWrapper;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use wallet_common::currencies::StatisticsItem;
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

    fn from_msgpack(mut pack: Value) -> Result<Self, Error> {
        match serde_json::from_value(pack["data"].take()) {
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
pub struct TransactionContextAck {
    pub exchangers: Vec<TransactionExchangerItem>,
}

impl TXMsgPackageData for TransactionContextAck {
    fn get_msgtype() -> String {
        "TransactionContextAck".to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyStat {
    pub statistics: Vec<StatisticsItem>,
}

impl TXMsgPackageData for CurrencyStat {
    fn get_msgtype() -> String {
        "CurrencyStat".to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyPlan {
    pub peer_plans: Vec<PeerCurrencyPlan>,
}

impl TXMsgPackageData for CurrencyPlan {
    fn get_msgtype() -> String {
        "CurrencyPlan".to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionSyn {
    pub tx_datas: Vec<String>,
}

impl TXMsgPackageData for TransactionSyn {
    fn get_msgtype() -> String {
        "TransactionSyn".to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionConfirm {}

impl TXMsgPackageData for TransactionConfirm {
    fn get_msgtype() -> String {
        "TransactionConfirm".to_string()
    }
}
