use crate::error::Error;
use crate::Bus;
use actix::prelude::*;
use serde_json::Value;

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<(), Error>")]
pub struct Event {
    pub addr: Addr<Bus>,
    pub id: u64,
    pub machine: String,
    pub event: String,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<Value, Error>")]
pub struct Call {
    pub addr: Addr<Bus>,
    pub method: String,
    pub args: Value,
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
