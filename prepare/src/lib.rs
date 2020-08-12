use serde_json::{Value, json};
use std::fmt;

use actix::prelude::*;
use ewf_core::error::Error as EwfError;
use ewf_core::{Bus, Call, Event, Module, StartNotify, Transition};
use ewf_core::{call_mod_througth_bus, call_self};
use std::collections::hash_map::HashMap;
use wallet_common::prepare::{ModStatus, ModStatusPullParam, ModInitialParam};
use wallet_common::WALLET_SM_CODE;

pub struct PrepareModule {
    bus_addr: Option<Addr<Bus>>,
    prepare_num_f: u64,
    prepare_num_s: u64,
    prepare_cnt: u64,
    prepare_map: HashMap<String, ModStatus>,

    mod_priority_min: i32,
    mod_priority_max: i32,
}

impl PrepareModule {
    pub fn new(mod_priority_min: i32, mod_priority_max: i32, prepare_mods: Vec<&str>) -> Self {
        let prepare_cnt = prepare_mods.len() as u64;
        let mut prepare_map = HashMap::<String, ModStatus>::new();
        prepare_mods
            .iter()
            .map(|x| prepare_map.insert(x.to_string(), ModStatus::Ignore))
            .count();

        Self {
            bus_addr: None,
            prepare_num_f: 0,
            prepare_num_s: 0,
            prepare_cnt: prepare_cnt,
            prepare_map,
            mod_priority_min,
            mod_priority_max,
        }
    }
}

impl Actor for PrepareModule {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {}
}

impl Handler<Call> for PrepareModule {
    type Result = ResponseFuture<Result<Value, EwfError>>;
    fn handle(&mut self, msg: Call, ctx: &mut Context<Self>) -> Self::Result {
        let bus_addr = self.bus_addr.clone().unwrap();
        let self_addr = ctx.address();
        let args = msg.args.clone();
        let mod_priority_min= self.mod_priority_min;
        let mod_priority_max= self.mod_priority_max;
        let mod_names: Vec<String> = self.prepare_map.keys().map(|k| k.clone()).collect();
        let mod_status = self.prepare_map.clone();
        let prepare_num_s = self.prepare_num_s;
        let prepare_cnt = self.prepare_cnt;

        let resolve_inital =
            move || -> Result<ResponseFuture<Result<Value, EwfError>>, EwfError> {
                let param: ModStatusPullParam =
                    serde_json::from_value(args).map_err(|_| EwfError::CallParamValidFaild)?;

                match param.is_prepare {
                    ModStatus::InitalSuccess => {
                        match self
                            .prepare_map
                            .insert(param.mod_name.clone(), ModStatus::InitalSuccess)
                        {
                            None => log::warn!("unknown mod {} initial", param.mod_name),
                            Some(ModStatus::Ignore) => {
                                self.prepare_num_s += 1;

                                log::info!("string...  {}     {:?}", param.mod_name, ModStatus::InitalSuccess);
                            },
                            Some(status) => log::warn!(
                                "mod {} initial success, but expect from status {:?}",
                                param.mod_name,
                                status
                            ),
                        }
                    }
                    ModStatus::InitalFailed => {
                        self.prepare_map
                            .insert(param.mod_name, ModStatus::InitalFailed);
                        self.prepare_num_f += 1;
                    }
                    _ => {}
                }
                
                Ok(Box::pin(async move { Ok(Value::Null) }))
            };

        let method: &str = &msg.method;
        match method {
            "mod_initial_return" => match resolve_inital() {
                Ok(result) => result,
                Err(err) => Box::pin(async move { Err(err) }),
            },
            "initial_controler_start" => Box::pin(async move {
                log::info!("initial_controler_start>>>>");
                for priority in mod_priority_min..mod_priority_max+1 {
                    for each_mod in &mod_names {
                        call_mod_througth_bus!(bus_addr, each_mod, "mod_initial", json!(ModInitialParam{priority: priority}));
                    }
                }

                call_self!(self_addr, "initial_controler_endcheck", Value::Null);

                Ok(Value::Null)
            }),
            "initial_controler_endcheck" => Box::pin(async move {
                log::info!("initial_controler_end<<<<");

                for (mod_name, status) in mod_status.into_iter() {
                    log::info!(" {}     {:?}", mod_name, status);
                }

                if prepare_num_s == prepare_cnt {
                    bus_addr.do_send(Transition {
                        id: WALLET_SM_CODE,
                        transition: "Starting".to_string(),
                    });
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
    fn handle(&mut self, msg: StartNotify, ctx: &mut Context<Self>) -> Self::Result {
        self.bus_addr = Some(msg.addr.clone());

        ctx.notify(Call{method: "initial_controler_start".to_string(), args: Value::Null});
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
