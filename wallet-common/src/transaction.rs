use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TXSendRequest {
    pub uid: String,
    pub oppo_peer_uid: String,
    pub exchangers: Vec<TransactionExchangerItem>,
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
    pub intput: u64,
    /// 预留字段
    pub addition: Value,
}
