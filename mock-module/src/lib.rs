use actix::prelude::*;
use ewf_core::message::CallQuery;
use ewf_core::{error::Error, Bus, Call, Event, Module, StartNotify};
use serde_json::Value;
use std::fmt::{self, Debug, Formatter};

pub struct MockModule {
    pub bus_addr: Option<Addr<Bus>>,
}

impl Actor for MockModule {
    type Context = Context<Self>;
}

impl Handler<Call> for MockModule {
    type Result = ResponseFuture<Result<Value, Error>>;
    fn handle(&mut self, _msg: Call, _ctx: &mut Context<Self>) -> Self::Result {
        let bus_addr = self.bus_addr.clone().unwrap();

        Box::pin(async move {
            let query = CallQuery {
                module: "mock-module".to_string(),
            };
            let _caller = bus_addr.send(query).await;
            println!("recv call");
            Ok(Value::default())
        })
    }
}

impl Handler<Event> for MockModule {
    type Result = Result<(), Error>;
    fn handle(&mut self, _msg: Event, _ctx: &mut Context<Self>) -> Self::Result {
        println!("recv event");
        Ok(())
    }
}

impl Handler<StartNotify> for MockModule {
    type Result = ();
    fn handle(&mut self, _msg: StartNotify, _ctx: &mut Context<Self>) -> Self::Result {
        self.bus_addr = Some(_msg.addr);
    }
}

impl Debug for MockModule {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_fmt(format_args!("mock-module"))
    }
}

impl Module for MockModule {
    fn name(&self) -> String {
        "mock-module".to_string()
    }

    fn version(&self) -> String {
        "0.1".to_string()
    }
}
