use crate::error::Error;
use crate::machines::{Machine, MachineManager};
use crate::message::{
    Call, CallQuery, CreateMachine, DestoryMachine, Event, StartNotify, Transition,
};
use crate::Module;
use actix::prelude::*;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};

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
    start_caller: HashMap<String, Recipient<StartNotify>>,
    priorities: Vec<PriorityPair>,
    addr: Option<Addr<Self>>,
}

impl Bus {
    fn crate_start_list(&self) -> Vec<(String, i32)> {
        self.priorities
            .iter()
            .map(|each| (each.1.clone(), each.0))
            .collect()
    }
}

impl Actor for Bus {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.priorities.sort();
        self.addr = Some(ctx.address());

        for pp in self.priorities.iter() {
            if let Some(caller) = self.start_caller.get(&pp.1) {
                caller
                    .do_send(StartNotify {
                        addr: ctx.address(),
                        start_list: self.crate_start_list(),
                    })
                    .unwrap();
            }
        }
    }
}

impl Debug for Bus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("bus"))
    }
}

impl Bus {
    pub fn new() -> Self {
        Self {
            machines: MachineManager::new(),
            call_caller: HashMap::new(),
            event_caller: HashMap::new(),
            start_caller: HashMap::new(),
            priorities: Vec::new(),
            addr: None,
        }
    }

    pub fn transite(&mut self, msg: Transition) -> Result<(), Error> {
        let (id, machine, event) = self.machines.transition(msg.id, msg.transition)?;
        log::info!("machine {}_{} transition => {}", machine, id, event);
        for pp in self.priorities.iter() {
            if let Some(caller) = self.event_caller.get(&pp.1) {
                caller.do_send(Event {
                    id,
                    machine: machine.clone(),
                    event: event.clone(),
                })?;
            }
        }
        Ok(())
    }

    pub fn get_caller(&self, module: String) -> Option<&Recipient<Call>> {
        self.call_caller.get(&module)
    }

    pub fn module<A>(&mut self, priority: i32, actor: A) -> &mut Self
    where
        A: Actor<Context = Context<A>>
            + Module
            + Handler<Call>
            + Handler<Event>
            + Handler<StartNotify>,
    {
        let name = actor.name();
        let addr = actor.start();
        let call_caller = addr.clone().recipient();
        let start_caller = addr.clone().recipient();
        let event_caller = addr.recipient();
        self.event_caller.insert(name.clone(), event_caller);
        self.call_caller.insert(name.clone(), call_caller);
        self.start_caller.insert(name.clone(), start_caller);
        self.priorities.push(PriorityPair(priority, name));
        self
    }

    pub fn machine<M: Machine + 'static>(&mut self, m: M) -> &mut Self {
        self.machines.insert(Box::new(m));
        self
    }
}

impl Handler<Transition> for Bus {
    type Result = Result<(), Error>;
    fn handle(&mut self, msg: Transition, _ctx: &mut Context<Self>) -> Self::Result {
        self.transite(msg)
    }
}

impl Handler<CallQuery> for Bus {
    type Result = Result<Recipient<Call>, Error>;
    fn handle(&mut self, msg: CallQuery, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(recipient) = self.call_caller.get(&msg.module) {
            Ok(recipient.clone())
        } else {
            Err(Error::NoModule)
        }
    }
}

impl Handler<CreateMachine> for Bus {
    type Result = u64;
    fn handle(&mut self, msg: CreateMachine, _ctx: &mut Context<Self>) -> Self::Result {
        self.machines.insert(msg.machine)
    }
}

impl Handler<DestoryMachine> for Bus {
    type Result = ();
    fn handle(&mut self, msg: DestoryMachine, _ctx: &mut Context<Self>) -> Self::Result {
        self.machines.delete(msg.machine_id)
    }
}
