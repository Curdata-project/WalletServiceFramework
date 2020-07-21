use crate::error::Error;
use actix::prelude::*;
use serde_json::Value;
use actix::Recipient;

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
    pub module: String,
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
#[rtype(result = "Recipient<Caller>")]
pub struct Caller {
    pub module: String,
}

