use crate::currencies::StatisticsItem;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TXSendRequest {
    pub uid: String,
    pub oppo_peer_uid: String,
    pub input: u64,
    pub output: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TXSendResponse {
    pub txid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TXCloseRequest {
    pub txid: String,
    pub uid: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionExchangerItem {
    pub uid: String,
    pub cert: String,
    pub output: u64,
    pub input: u64,
    /// 预留字段
    pub addition: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyPlanItem {
    pub pay_amount: u64,
    pub pay_plan: Vec<StatisticsItem>,
    pub recv_amount: u64,
    pub recv_plan: Vec<StatisticsItem>,
}
