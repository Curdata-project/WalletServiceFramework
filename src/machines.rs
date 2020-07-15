use crate::error::Error;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;

#[derive(Debug)]
pub struct Event {
    pub id: u64,
    pub machine: String,
    pub event: String,
}

pub trait Machine {
    fn to_string(&self) -> String;

    fn name(&self) -> String;

    fn transition(&mut self, t: String) -> Result<String, Error>;
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

    pub fn transition(&mut self, id: u64, t: String) -> Result<Event, Error> {
        if let Some(machine) = self.machines.get_mut(&id) {
            let event = machine.transition(t)?;
            let name = machine.name();
            Ok(Event {
                event,
                id,
                machine: name,
            })
        } else {
            Err(Error::NoStateMachine)
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
