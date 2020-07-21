use actix::prelude::*;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use serde_json::Value;

use crate::error::Error;
use crate::machines::MachineManager;
use crate::message::{Transition, Caller, Event, Call};

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
    call_caller: HashMap<String, Recipient<Call>>,
    event_caller: HashMap<String, Recipient<Event>>,
    priorities: BinaryHeap<PriorityPair>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            machines: MachineManager::new(),
            call_caller: HashMap::new(),
            event_caller: HashMap::new(),
            priorities: BinaryHeap::new(),
        }
    }

    fn transite(&mut self, msg: Transition) -> Result<(), Error> {
        let event = self.machines.transition(msg.id, msg.transition)?;
        for pp in self.priorities.iter() {
            if let Some(caller) = self.event_caller.get(&pp.1) {
                caller.do_send(event.clone())?;
            }
        }
        Ok(())
    }

    fn get_caller(&self, module: String) -> Option<&Recipient<Call>> {
        self.call_caller.get(&module)
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

impl Handler<Caller> for Bus {
    type Result = Result<Recipient<Call>, Error>;
    fn handle(&mut self, msg: Caller, _ctx: &mut Context<Self>) -> Self::Result {
        self.call_caller.get(&msg.module)
    }
}
