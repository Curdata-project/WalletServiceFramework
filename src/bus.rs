use alloc::collections::BTreeMap;
use serde_json::Value;
use alloc::string::String;
use alloc::boxed::Box;
use crate::MachineManager;
use crate::Event;
use crate::Machine;

pub trait Module{
    /// 负责传递状态到下一环
    fn event_call(&self, bus: &Bus, event: &Event);

    /// 负责外部状态触发与函数名及可变性的映射
    fn call(&self, method: &str, intput: Value);
}

pub struct Bus {
    mods: BTreeMap<String, Box<dyn Module + 'static>>,
    priorities: BTreeMap<i32, String>,
    machines: MachineManager,
}

impl Bus {
    pub fn new() -> Self {
        Self{
            mods: BTreeMap::new(),
            priorities: BTreeMap::new(),
            machines: MachineManager::new(),
        }
    }

    pub fn registe_module(mut self, name: String, priority: i32, module: Box<dyn Module + 'static>) -> Self {
        self.mods.insert(name.clone(), module);
        self.priorities.insert(priority, name);
        self
    }

    pub fn get_module(&self, name: &str) -> Option<&Box<dyn Module + 'static>> {
        self.mods.get(name)
    }

    pub fn event_call(&self, event: &Event) {
        for (_, n) in self.priorities.iter() {
            let m = self.mods.get(n);
            if m.is_some() {
                m.unwrap().event_call(self, event);
            }
        }
    }

    pub fn transition(&mut self, id: u64, t: String) -> Result<(), ()> {
        let r = self.machines.transition(id, t);
        if r.is_ok() {
            let event = r.unwrap();
            let e = Event {
                id,
                event,
            };
            self.event_call(&e);
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn registe_machine(mut self, machine: Box<dyn Machine>) -> Self {
        self.machines.insert(machine);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::states::WalletMachine;
    use alloc::string::ToString;

    struct MockModule;

    impl Module for MockModule {
        fn event_call(&self, bus: &Bus, event: &Event) {
            log::info!("{:?}", event);
            let m = bus.get_module("mock").unwrap();
            m.call("asda", Value::default());
        }
    
        fn call(&self, method: &str, _intput: Value) {
            log::info!("call: {}", method);
        }
    }
    #[test]
    fn t() {
        use env_logger::Env;
        env_logger::from_env(Env::default().filter("info")).init();

        let wallet_state = WalletMachine::default();
        let mut bus = Bus::new().registe_machine(Box::new(wallet_state))
            .registe_module("mock".to_string(), 1, Box::new(MockModule));
        let r = bus.transition(0, "".to_string());
        log::info!("{:?}", r);
    }
}