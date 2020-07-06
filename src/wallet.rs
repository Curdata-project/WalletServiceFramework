use crate::currency_store::CurrencyStore;
use std::sync::{Arc, RwLock};

pub struct Wallet {
    pub currency_store: RwLock<CurrencyStore>,
}

impl Wallet {
    pub fn new() -> Self {
        Self {
            currency_store: RwLock::new(CurrencyStore {}),
        }
    }
}
