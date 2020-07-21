use crate::error::Error;
use crate::machines::{Machine, MachineManager};
use crate::message::{Call, Event, Transition};
use crate::Module;
use actix::prelude::*;
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::HashMap;

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

    pub fn transite(&mut self, msg: Transition) -> Result<(), Error> {
        let event = self.machines.transition(msg.id, msg.transition)?;
        for pp in self.priorities.iter() {
            if let Some(caller) = self.event_caller.get(&pp.1) {
                caller.do_send(event.clone())?;
            }
        }
        Ok(())
    }

    pub fn get_caller(&self, module: String) -> Option<&Recipient<Call>> {
        self.call_caller.get(&module)
    }

    pub async fn call(&self, module: String, method: String, args: Value) -> Result<Value, Error> {
        if let Some(recipient) = self.call_caller.get(&module) {
            let call = Call { method, args };
            recipient.send(call).await?
        } else {
            Err(Error::NoModule)
        }
    }

    pub fn module<A>(&mut self, actor: A) -> &mut Self
    where
        A: Actor<Context = Context<A>> + Module + Handler<Call> + Handler<Event>,
    {
        let name = actor.name();
        let addr = actor.start();
        let call_caller = addr.clone().recipient();
        let event_caller = addr.recipient();
        self.event_caller.insert(name.clone(), event_caller);
        self.call_caller.insert(name, call_caller);
        self
    }

    pub fn machine<M: Machine + 'static>(&mut self, m: M) -> &mut Self {
        self.machines.insert(Box::new(m));
        self
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
