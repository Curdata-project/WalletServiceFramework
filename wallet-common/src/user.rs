use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEntity {
    pub uid: String,
    pub cert: String,
    pub last_tx_time: i64,
    pub account: String,
}
