use crate::module_bus::{Module, ModuleBus};
use std::sync::{Weak, RwLock, Arc};
use serde_json::{json, Value};
use crate::storage::currency_store::CurrencyStore;


pub struct CurrencyStoreAtWallet{
    module_bus: Weak<ModuleBus>,

    inner: Arc<RwLock<CurrencyStore>>,
}

impl CurrencyStoreAtWallet {
    pub fn new(module_bus: Weak<ModuleBus>, inner: Arc<RwLock<CurrencyStore>>,) -> Self {
        Self {
            module_bus,
            inner
        }
    }
}

impl Module for CurrencyStoreAtWallet {

    fn event_call(&self, status: &str) {
        let module_bus = if let Some(module_bus) = self.module_bus.upgrade() {
            module_bus
        }
        else {
            log::error!("module_bus cant get ref from weak");
            return;
        };

        match status {
            "Start" => {
                let params = json!({"test": 12});
                let event = "add_currency";

                for each in module_bus.get_module(&"key_store") {
                    each.call(&event, params.clone());
                }
            }
            _ => {},
        }
    }

    
    fn call(&self, event: &str, intput: Value) {
        match event {
            "init" => {
                let mut currency = self.inner.write().unwrap();
                currency.init();
            },
            _ => {

            },
        }
    }
}