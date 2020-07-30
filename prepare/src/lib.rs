use serde_json::Value;
use std::fmt;

use actix::prelude::*;
use ewf_core::error::Error as EwfError;
use ewf_core::{Bus, Call, CallQuery, Event, Module, StartNotify, Transition};
use wallet_common::prepare::{ModStatusPullParam, ModStatus};
use wallet_common::WALLET_SM_CODE;
use std::collections::hash_map::HashMap;


pub struct PrepareModule {
    bus_addr: Option<Addr<Bus>>,
    prepare_num_f: u64,
    prepare_num_s: u64,
    prepare_cnt: u64,
    prepare_map: HashMap<String, ModStatus>,
}

impl PrepareModule {
    pub fn new(prepare_mods: Vec<&str>) -> Self {
        let prepare_cnt = prepare_mods.len() as u64;
        let mut prepare_map = HashMap::<String, ModStatus>::new();
        prepare_mods.iter().map(|x| prepare_map.insert(x.to_string(), ModStatus::UnInital)).count();

        Self {
            bus_addr: None,
            prepare_num_f: 0,
            prepare_num_s: 0,
            prepare_cnt: prepare_cnt,
            prepare_map,
        }
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
        let args = msg.args.clone();

        let resolve_inital =
            move || -> Result<ResponseFuture<Result<Value, EwfError>>, EwfError> {
                let param: ModStatusPullParam =
                    serde_json::from_value(args).map_err(|_| EwfError::CallParamValidFaild)?;

                match param.is_prepare{
                    ModStatus::InitalSuccess => {
                        match self.prepare_map.insert(param.mod_name.clone(), ModStatus::InitalSuccess) {
                            None => log::warn!("unknown mod {} initialization", param.mod_name),
                            Some(ModStatus::UnInital) => self.prepare_num_s += 1,
                            Some(status) => log::warn!("mod {} initial success, but expect from status {:?}", param.mod_name, status),
                        }
                    },
                    ModStatus::InitalFailed => {
                        self.prepare_map.insert(param.mod_name, ModStatus::InitalFailed);
                        self.prepare_num_f += 1;
                    }
                    _ => { },
                }

                if self.prepare_num_s + self.prepare_num_f == self.prepare_cnt {
                    let inital_ans = match self.prepare_num_s == self.prepare_cnt {
                        true => "InitalSuccess".to_string(),
                        false => "InitalFailed".to_string(),
                    };
                    let inital_task = Box::pin(async move {
                        bus_addr
                            .send(Transition {
                                id: WALLET_SM_CODE,
                                transition: inital_ans,
                            })
                            .await??;
                        Ok(Value::Null)
                    });

                    return Ok(inital_task);
                }
                Ok(Box::pin(async move { Ok(Value::Null) }))
            };

        let method: &str = &msg.method;
        match method {
            "inital" => match resolve_inital() {
                Ok(result) => result,
                Err(err) => Box::pin(async move { Err(err) }),
            },
            _ => return Box::pin(async move { Err(EwfError::MethodNotFoundError) }),
        }
    }
}

impl Handler<Event> for PrepareModule {
    type Result = ResponseFuture<Result<(), EwfError>>;
    fn handle(&mut self, msg: Event, _ctx: &mut Context<Self>) -> Self::Result {
        let event: &str = &msg.event;
        let id = msg.id;
        match event {
            "Start" => {
                self.prepare_num_s = 0;
                self.prepare_num_f = 0;
            }
            // no care this event, ignore
            _ => {}
        }

        Box::pin(async { Ok(()) })
    }
}

impl Handler<StartNotify> for PrepareModule {
    type Result = ();
    fn handle(&mut self, msg: StartNotify, _ctx: &mut Context<Self>) -> Self::Result {
        self.bus_addr = Some(msg.addr.clone());

        msg.addr.do_send(Transition {
            id: WALLET_SM_CODE,
            transition: "Starting".to_string(),
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
