use actix::prelude::*;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::HashMap;

use crate::error::Error;
use crate::machines::MachineManager;
use crate::message::Transition;
use crate::Module;

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
    machines: MachineManager,
    mods: HashMap<String, Box<dyn Module + 'static>>,
    priorities: BinaryHeap<PriorityPair>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            machines: MachineManager::new(),
            mods: HashMap::new(),
            priorities: BinaryHeap::new(),
        }
    }

    fn transite(&mut self, msg: Transition) -> Result<(), Error> {
        let event = self.machines.transition(msg.id, msg.transition)?;
        for pp in self.priorities.iter() {
            let m = self.mods.get(&pp.1);
            if m.is_some() {
                m.unwrap().notify(&event)?;
            }
        }
        Ok(())
    }
}

impl Actor for Bus {
    type Context = Context<Self>;
}

impl Handler<Transition> for Bus {
    type Result = Result<(), Error>;
    fn handle(&mut self, msg: Transition, _ctx: &mut Context<Self>) -> Self::Result {
        self.transite(msg)
    }
}
