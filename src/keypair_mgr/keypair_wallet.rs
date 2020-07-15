use crate::module_bus::{Module, ModuleBus};
use std::sync::{Weak, RwLock, Arc};
use serde_json::{json, Value};
use crate::storage::keypair_store::KeypairStore;


pub struct KeypairStoreAtWallet{
    module_bus: Weak<ModuleBus>,

    inner: Arc<RwLock<KeypairStore>>,
}

impl KeypairStoreAtWallet {
    pub fn new(module_bus: Weak<ModuleBus>, inner: Arc<RwLock<KeypairStore>>,) -> Self {
        Self {
            module_bus,
            inner
        }
    }
}

impl Module for KeypairStoreAtWallet {

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
            "add_currency" => {
                let currency = self.inner.write().unwrap();
            },
            _ => {

            },
        }
    }
}