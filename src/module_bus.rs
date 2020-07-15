use alloc::collections::BTreeMap;
use std::collections::hash_map::HashMap;
use std::sync::{Arc, RwLock, Weak};
use serde_json::{Value, json};


pub trait Module{
    /// 负责传递状态到下一环
    fn event_call(&self, status: &str);

    /// 负责外部状态触发与函数名及可变性的映射
    fn call(&self, event: &str, intput: Value);
}

pub struct ModuleBus {
    mods: HashMap<String, BTreeMap<u16, Box<dyn Module + 'static>>>,
}

impl ModuleBus {
    pub fn new() -> Self {
        Self{
            mods: HashMap::new(),
        }
    }

    pub fn registe_module(mut self, name: String, priority: u16, module: Box<dyn Module + 'static>) -> Self {
        if let Some(bucket) = self.mods.get_mut(&name) {
            bucket.insert(priority,  module);
        }
        else{
            let mut new_bucket = BTreeMap::new();
            new_bucket.insert(priority,  module);

            self.mods.insert(name, new_bucket);
        };

        self
    }

    pub fn get_module(&self, name: &str) -> Vec<Arc<Box<dyn Module + 'static>>> {
        if let Some(mods) = self.mods.get(name) {
            mods.iter().map(|each| each.1.clone()).collect()
        }
        else{
            Vec::new()
        }
    }
}