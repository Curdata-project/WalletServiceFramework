use crate::Error;
use crate::Event;
use crate::Machine;
use crate::MachineManager;
use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BinaryHeap};
use alloc::string::String;
use core::cmp::{Ord, Ordering};
use serde_json::Value;

pub trait Module {
    fn event_call(&self, bus: &Bus, event: &Event) -> Result<(), Error>;

    fn call(&self, method: &str, intput: Value) -> Result<Value, Error>;

    fn name(&self) -> String;

    fn version(&self) -> String;
}

struct PriorityPair(pub i32, pub String);

impl PartialEq for PriorityPair {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd for PriorityPair {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Eq for PriorityPair {}

impl Ord for PriorityPair {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

pub struct Bus {
    mods: BTreeMap<String, Box<dyn Module + 'static>>,
    priorities: BinaryHeap<PriorityPair>,
    machines: MachineManager,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            mods: BTreeMap::new(),
            priorities: BinaryHeap::new(),
            machines: MachineManager::new(),
        }
    }

    pub fn registe_module(mut self, priority: i32, module: Box<dyn Module + 'static>) -> Self {
        let name = module.name();
        self.mods.insert(name.clone(), module);
        self.priorities.push(PriorityPair(priority, name));
        self
    }

    pub fn registe_machine(mut self, machine: Box<dyn Machine>) -> Self {
        self.machines.insert(machine);
        self
    }

    pub fn get_module(&self, name: &str) -> Option<&Box<dyn Module + 'static>> {
        self.mods.get(name)
    }

    pub(crate) fn event_call(&self, event: &Event) -> Result<(), Error> {
        for pp in self.priorities.iter() {
            log::info!("event_call: {} {:?}", &pp.1, event);

            let m = self.mods.get(&pp.1);
            if m.is_some() {
                m.unwrap().event_call(self, event)?;
            }
        }
        Ok(())
    }

    pub fn transition(&mut self, id: u64, t: String) -> Result<(), Error> {
        let event = self.machines.transition(id, t)?;
        self.event_call(&event)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::states::WalletMachine;
    use alloc::string::ToString;

    struct MockModule;

    impl Module for MockModule {
        fn event_call(&self, bus: &Bus, event: &Event) -> Result<(), Error> {
            log::info!("{:?}", event);
            let m = bus.get_module("mock").unwrap();
            m.call("asda", Value::default())?;
            Ok(())
        }

        fn call(&self, method: &str, _intput: Value) -> Result<Value, Error> {
            log::info!("call: {}", method);
            Ok(Value::default())
        }

        fn name(&self) -> String {
            "mock".to_string()
        }

        fn version(&self) -> String {
            "0.1.0".to_string()
        }
    }
    #[test]
    fn t() {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Info)
            .try_init();

        let wallet_state = WalletMachine::default();
        let mut bus = Bus::new()
            .registe_machine(Box::new(wallet_state))
            .registe_module(1, Box::new(MockModule));
        let r = bus.transition(0, "Starting".to_string());
        log::info!("{:?}", r);
    }
}
