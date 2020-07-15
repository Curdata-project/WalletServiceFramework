use crate::module_bus::{Module, ModuleBus};
use std::sync::{Arc, Weak, RwLock};
use serde_json::{json, Value};
use crate::storage::currency_store::CurrencyStore;
use crate::storage::keypair_store::KeypairStore;
use crate::currency_mgr::currency_wallet::CurrencyStoreAtWallet;
use crate::keypair_mgr::keypair_wallet::KeypairStoreAtWallet;
use rustorm::Pool;


pub struct Wallet{
    currency_store: Arc<RwLock<CurrencyStore>>,
    keypair_store: Arc<RwLock<KeypairStore>>,

    main_state_machine: Arc<ModuleBus>,
}

impl Wallet{
    pub fn build(path: String) -> Arc<ModuleBus> {
        let module_bus = Arc::new(ModuleBus::new());
        let module_bus_weak = Arc::downgrade(&module_bus);
    
        let mut wallet = WalletMgr{
            module_bus: module_bus_weak,
            pool: Pool::new(),
            path: path.clone(),
        };
    
        // 暂预设wallet_mgr没有运行时mut行为，只是启动器
        module_bus.registe_module("wallet_mgr".to_string(), 0, Box::new(wallet));
        module_bus.registe_module("currency_mgr".to_string(), 0, 
        Box::new(CurrencyStoreAtWallet::new(module_bus_weak, Arc::new(RwLock::new(CurrencyStore::new(path.clone()))))));
        module_bus.registe_module("keypair_mgr".to_string(), 0, 
        Box::new(KeypairStoreAtWallet::new(module_bus_weak, Arc::new(RwLock::new(KeypairStore::new(path.clone()))))));
    
        module_bus
    }
}


pub(crate) struct WalletMgr {
    module_bus: Weak<ModuleBus>,

    pool: Pool,
    path: String,
}

impl WalletMgr {
    
}

impl Module for WalletMgr {

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
                for each in module_bus.get_module(&"currency_mgr") {
                    each.call(&"init", Value::Null);
                }
            }
            _ => {},
        }
    }

    
    fn call(&self, event: &str, intput: Value) {
        match event {
            "End" => {
            },
            _ => {

            },
        }
    }
}