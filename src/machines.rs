use alloc::string::String;
use alloc::collections::BTreeMap;
use alloc::boxed::Box;

#[derive(Debug)]
pub struct Event {
    pub id: u64,
    pub event: String,
}

pub trait Machine {
    fn to_string(&self) -> String;

    fn transition(&mut self, t: String) -> Result<String, ()>;
}

pub struct MachineManager {
    machines: BTreeMap<u64, Box<dyn Machine>>,
    count: u64,
}

impl MachineManager {
    pub fn new() -> Self {
        Self {
            machines: BTreeMap::new(),
            count: 0,
        }
    }

    pub fn transition(&mut self, id: u64, t: String) -> Result<String, ()> {
        let m = self.machines.get_mut(&id);
        if m.is_some() {
            m.unwrap().transition(t)
        } else {
            Err(())
        }
    }

    pub fn insert(&mut self, machine: Box<dyn Machine>) {
        let id = self.count;
        self.count = id + 1;
        self.machines.insert(id, machine);
    }

    pub fn get(&self, id: &u64) -> Option<&Box<dyn Machine>> {
        self.machines.get(&id)
    }
}
