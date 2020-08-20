use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TXSendRequest {
    pub uid: String,
    pub oppo_peer_uid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TXSendResponse {
    pub txid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TXCloseRequest {
    pub txid: String,
}
