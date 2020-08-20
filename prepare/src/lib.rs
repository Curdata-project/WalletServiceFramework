use serde_json::{json, Value};
use std::fmt;

use actix::prelude::*;
use ewf_core::error::Error as EwfError;
use ewf_core::{async_parse_check, call_mod_througth_bus};
use ewf_core::{Bus, Call, Event, Module, StartNotify, Transition};
use serde::{Deserialize, Serialize};
use wallet_common::prepare::{ModInitialParam, ModStatus};
use wallet_common::WALLET_SM_CODE;

pub struct PrepareModule {
    bus_addr: Option<Addr<Bus>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InitialControlerStartParam {
    start_list: Vec<(String, i32)>,
}

impl PrepareModule {
    pub fn new() -> Self {
        Self { bus_addr: None }
    }
}

impl Actor for PrepareModule {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {}
}

impl Handler<Call> for PrepareModule {
    type Result = ResponseFuture<Result<Value, EwfError>>;
    fn handle(&mut self, msg: Call, _ctx: &mut Context<Self>) -> Self::Result {
        let bus_addr = self.bus_addr.clone().unwrap();
        let self_name = self.name();

        let method: &str = &msg.method;
        match method {
            "initial_controler_start" => Box::pin(async move {
                let params: InitialControlerStartParam =
                    async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                log::info!("initial_controler_start>>>>");
                let mut start_success = true;
                for (mod_name, _priority) in params.start_list {
                    if mod_name == self_name {
                        log::info!("skiping...  {}     ", mod_name);
                        continue;
                    }
                    let ans = call_mod_througth_bus!(
                        bus_addr,
                        mod_name,
                        "mod_initial",
                        json!(ModInitialParam {})
                    );

                    let is_initialed: ModStatus =
                        async_parse_check!(ans, EwfError::CallParamValidFaild);
                    start_success = start_success | (ModStatus::InitalSuccess == is_initialed);

                    log::info!("starting...  {}     {:?}", mod_name, is_initialed);
                }
                log::info!("initial_controler_end>>>>");

                if start_success {
                    bus_addr.do_send(Transition {
                        id: WALLET_SM_CODE,
                        transition: "Starting".to_string(),
                    });
                } else {
                    // TODO 启动失败
                }

                Ok(Value::Null)
            }),
            _ => return Box::pin(async move { Err(EwfError::MethodNotFoundError) }),
        }
    }
}

impl Handler<Event> for PrepareModule {
    type Result = ResponseFuture<Result<(), EwfError>>;
    fn handle(&mut self, msg: Event, _ctx: &mut Context<Self>) -> Self::Result {
        let event: &str = &msg.event;
        match event {
            "Start" => {}
            // no care this event, ignore
            _ => {}
        }

        Box::pin(async { Ok(()) })
    }
}

impl Handler<StartNotify> for PrepareModule {
    type Result = ();
    fn handle(&mut self, msg: StartNotify, ctx: &mut Context<Self>) -> Self::Result {
        self.bus_addr = Some(msg.addr.clone());

        ctx.notify(Call {
            method: "initial_controler_start".to_string(),
            args: json!(InitialControlerStartParam {
                start_list: msg.start_list
            }),
        });
    }
}

impl fmt::Debug for PrepareModule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("{{ {} {} }}", self.name(), self.version()))
    }
}

impl Module for PrepareModule {
    fn name(&self) -> String {
        "prepare".to_string()
    }

    fn version(&self) -> String {
        "0.1".to_string()
    }
}
