use crate::error::WallerError;
use crate::wallet::Wallet;
use jsonrpc_ws::Data;
use serde::{Deserialize, Serialize};

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
    pub fn get_detail_by_ids(&self) -> Result<Vec<CurrencyDetail>, WallerError> {
        Ok(Vec::<CurrencyDetail>::new())
    }
}

#[derive(Serialize, Deserialize)]
pub struct GetDetailParam {
    ids: Vec<String>,
}

pub async fn get_detail_by_ids(
    wallet: Data<Wallet>,
    req: GetDetailParam,
) -> Result<Vec<CurrencyDetail>, WallerError> {
    let currency_store = wallet.get_ref().currency_store.try_read().unwrap();
    currency_store.get_detail_by_ids()
}
