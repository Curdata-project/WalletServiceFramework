use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TXSendResponse {
    pub txid: String,
    pub conn_info: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TXCloseRequest {
    pub txid: String,
}
