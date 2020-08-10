use crate::error::Error;
use crate::machines::Machine;
use crate::Bus;
use actix::prelude::*;
use serde_json::Value;
use std::fmt;

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<(), Error>")]
pub struct Event {
    pub id: u64,
    pub machine: String,
    pub event: String,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<Value, Error>")]
pub struct Call {
    pub method: String,
    pub args: Value,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct StartNotify {
    pub addr: Addr<Bus>,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<(), Error>")]
pub struct Transition {
    pub id: u64,
    pub transition: String,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<Recipient<Call>, Error>")]
pub struct CallQuery {
    pub module: String,
}

#[derive(Message)]
#[rtype(result = "u64")]
pub struct CreateMachine {
    pub machine: Box<dyn Machine + Send>,
}

impl fmt::Debug for CreateMachine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!(" CreateMachine "))
    }
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct DestoryMachine {
    pub machine_id: u64,
}
