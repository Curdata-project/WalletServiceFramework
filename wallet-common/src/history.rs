use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransType {
    /// 充值
    Recharge,
    /// 提现
    Withdraw,
    /// 付款
    Pay,
    /// 收款
    Recv,
}

impl TransType {
    pub fn to_int16(&self) -> i16 {
        match self {
            TransType::Recharge => 0,
            TransType::Withdraw => 1,
            TransType::Pay => 2,
            TransType::Recv => 3,
        }
    }
    pub fn from_int16(a: i16) -> TransType {
        match a {
            0 => TransType::Recharge,
            1 => TransType::Withdraw,
            2 => TransType::Pay,
            3 => TransType::Recv,
            _ => TransType::Recharge,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntity {
    pub uid: String,
    pub txid: String,
    pub trans_type: TransType,
    pub oppo_uid: String,
    pub occur_time: i64,
    pub amount: u64,
    pub balance: u64,
    pub remark: String,
}
