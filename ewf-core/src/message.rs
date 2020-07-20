use crate::error::Error;
use actix::prelude::*;
use serde_json::Value;

#[derive(Debug, Message)]
#[rtype(result = "Result<(), Error>")]
pub struct Event {
    pub id: u64,
    pub machine: String,
    pub event: String,
}

#[derive(Debug, Message)]
#[rtype(result = "Result<Value, Error>")]
pub struct Call {
    pub method: String,
    pub args: Value,
}

#[derive(Debug, Message)]
#[rtype(result = "Result<(), Error>")]
pub struct Transition {
    pub id: u64,
    pub transition: String,
}
