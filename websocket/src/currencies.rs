use serde::{Serialize, Deserialize};
use crate::error::WalletError;
use jsonrpc_core::Data;
use ewf_core::{Call, Event, Module, Transition};
use actix::prelude::*;


#[derive(Serialize, Deserialize)]
pub struct CurrencyDetail {
    value: u64,
    id: String,
    dcds: String,
    locked: bool,
    owner: String,
}

pub struct CurrencyStore {}
impl CurrencyStore {
    pub fn get_detail_by_ids(
        &self,
        req: GetDetailParam,
    ) -> Result<Vec<CurrencyDetail>, WalletError> {
        if req.ids.len() == 0 {
            return Err(WalletError::ParamIsNone);
        }
        Ok(Vec::<CurrencyDetail>::new())
    }
}

#[derive(Serialize, Deserialize)]
pub struct GetDetailParam {
    ids: Vec<String>,
}

pub async fn get_detail_by_ids(
    wallet: Data<CurrencyResource>,
    req: GetDetailParam,
) -> Result<Vec<CurrencyDetail>, WalletError> {
    Err(WalletError::ParamIsNone)
}

pub struct CurrencyResource{
    currencies: Recipient<Call>
}

/// 事实上不会调用CurrencyResource.xxx，只是作为数据载体复制后在多个Future中使用
///   只要成员currencies支持Send即可
unsafe impl Sync for CurrencyResource {}

impl CurrencyResource {
    pub fn new(currencies: Recipient<Call>) -> Self {
        Self {
            currencies
        }
    }
}