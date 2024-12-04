use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindTransPortParam {
    pub uid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseBindTransPortParam {
    pub uid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectRequest {
    pub uid: String,
    pub oppo_peer_uid: String,
    pub txid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnConnectNotify {
    pub uid: String,
    pub oppo_peer_uid: String,
    pub txid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseConnectRequest {
    pub uid: String,
    pub txid: String,
}

pub type TransactionType = Vec<u8>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgPackage {
    pub txid: String,
    pub data: TransactionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMsgPackage {
    pub msg: MsgPackage,
    pub send_uid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecvMsgPackage {
    pub msg: MsgPackage,
    pub recv_uid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    pub uid: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpWrapPackage {
    pub msg: MsgPackage,
    pub ord_id: u32,
}

