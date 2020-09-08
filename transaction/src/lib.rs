use std::fmt;

use actix::prelude::*;
use chrono::prelude::Local;
use common_structure::digital_currency::DigitalCurrencyWrapper;
use common_structure::get_rng_core;
use ewf_core::error::Error as EwfError;
use ewf_core::states::TransactionMachine;
use ewf_core::{async_parse_check, call_mod_througth_bus, call_self, sync_parse_check};
use ewf_core::{Bus, Call, CreateMachine, Event, Module, StartNotify};
use hex::ToHex;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::hash_map::HashMap;
use std::time::Duration;
use wallet_common::connect::{CloseConnectRequest, ConnectRequest};
use wallet_common::prepare::{ModInitialParam, ModStatus};
use wallet_common::transaction::{TXCloseRequest, TXSendRequest, TXSendResponse};

const CHECK_CLOSE_INTERVAL: u64 = 3;
const MAX_CLOSE_TIME_MS: i64 = 2000;

/// 仅用作模块内互调参数转换错误之类的异常，确保是由编码错误触发
const BUG_ERROR_PANIC: &str = "found a bug";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TXSendSaveInput {
    pub tx_sm_id: u64,
}

impl From<Value> for TXSendSaveInput {
    fn from(input: Value) -> Self {
        serde_json::from_value(input).expect(BUG_ERROR_PANIC)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TXSendSaveOutput {
    pub txid: String,
}

impl From<Value> for TXSendSaveOutput {
    fn from(input: Value) -> Self {
        serde_json::from_value(input).expect(BUG_ERROR_PANIC)
    }
}

pub struct TransactionPayload {
    pub uid: String,
    pub is_payer: bool,
    pub amount: u64,
    pub oppo_uid: String,
    pub pay_plan: Vec<(u64, u64)>,
    pub recv_plan: Vec<(u64, u64)>,
    pub pay_list: Vec<DigitalCurrencyWrapper>,

    // 使用txid与conn管理交互
    pub txid: String,

    pub tx_sm_id: u64,
    pub last_update_time: i64,
}

impl TransactionPayload {
    fn new(tx_sm_id: u64, txid: String) -> Self {
        Self {
            uid: "".to_string(),
            is_payer: false,
            amount: 0,
            oppo_uid: "".to_string(),
            pay_plan: Vec::<(u64, u64)>::new(),
            recv_plan: Vec::<(u64, u64)>::new(),
            pay_list: Vec::<DigitalCurrencyWrapper>::new(),
            txid,
            tx_sm_id,
            last_update_time: Local::now().timestamp_millis(),
        }
    }

    fn gen_txid() -> String {
        let ret = Local::now().timestamp().to_string();

        // TODO 或许要考虑流程取出碰撞
        let mut arr = Vec::<u8>::new();
        arr.resize(8, 0);
        get_rng_core().fill_bytes(&mut arr[0..8]);
        ret + &arr.encode_hex::<String>()
    }
}

pub struct TransactionModule {
    bus_addr: Option<Addr<Bus>>,
    tx_sm_datas: HashMap<u64, TransactionPayload>,
    tx_link: HashMap<String, u64>,
}

impl TransactionModule {
    pub fn new() -> Self {
        Self {
            bus_addr: None,
            tx_sm_datas: HashMap::<u64, TransactionPayload>::new(),
            tx_link: HashMap::<String, u64>::new(),
        }
    }
}

impl Actor for TransactionModule {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {}
}

impl Handler<Call> for TransactionModule {
    type Result = ResponseFuture<Result<Value, EwfError>>;
    fn handle(&mut self, msg: Call, ctx: &mut Context<Self>) -> Self::Result {
        let bus_addr = self.bus_addr.clone().unwrap();
        let self_addr = ctx.address();

        let method: &str = &msg.method;
        match method {
            "mod_initial" => Box::pin(async move {
                let _params: ModInitialParam =
                    async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                Ok(json!(ModStatus::InitalSuccess))
            }),
            "tx_send" => Box::pin(async move {
                let params: TXSendRequest =
                    async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                // 创建状态机
                let tx_sm_id = bus_addr
                    .send(CreateMachine {
                        machine: Box::new(TransactionMachine::default()),
                    })
                    .await?;

                let save_ans: TXSendSaveOutput =
                    call_self!(self_addr, "tx_save_cb", json!(TXSendSaveInput { tx_sm_id })).into();

                // 建立链接，若此处连接失败，创建的状态机等随超时回收
                call_mod_througth_bus!(
                    bus_addr,
                    "tx_conn",
                    "connect",
                    json!(ConnectRequest {
                        uid: params.uid,
                        oppo_peer_uid: params.oppo_peer_uid,
                        txid: save_ans.txid.clone(),
                    })
                );
                log::info!("tx_connect {}", save_ans.txid.clone());

                Ok(json!(TXSendResponse {
                    txid: save_ans.txid,
                }))
            }),
            "tx_save_cb" => {
                let params: TXSendSaveInput =
                    sync_parse_check!(msg.args, EwfError::CallParamValidFaild);
                let new_tx_id = TransactionPayload::gen_txid();
                self.tx_sm_datas.insert(
                    params.tx_sm_id,
                    TransactionPayload::new(params.tx_sm_id, new_tx_id.clone()),
                );
                self.tx_link.insert(new_tx_id.clone(), params.tx_sm_id);

                Box::pin(async move { Ok(json!(TXSendSaveOutput { txid: new_tx_id })) })
            }
            "run_close_check_task" => {
                for tx_sm_id in self.tx_link.values() {
                    let pay_load = match self.tx_sm_datas.get(&tx_sm_id) {
                        Some(pay_load) => pay_load,
                        None => continue,
                    };

                    let now = Local::now().timestamp_millis();

                    // 关闭死链接
                    if now - pay_load.last_update_time > MAX_CLOSE_TIME_MS {
                        ctx.notify(Call {
                            method: "tx_close".to_string(),
                            args: json!(TXCloseRequest {
                                uid: pay_load.uid.clone(),
                                txid: pay_load.txid.clone(),
                                reason: "timeout".to_string(),
                            }),
                        });
                    }
                }

                Box::pin(async move { Ok(Value::Null) })
            }
            "tx_close" => {
                let params: TXCloseRequest =
                    sync_parse_check!(msg.args, EwfError::CallParamValidFaild);
                if let Some(tx_sm_id) = self.tx_link.get_mut(&params.txid) {
                    self.tx_sm_datas.remove(&tx_sm_id);
                }
                self.tx_link.remove(&params.txid);

                Box::pin(async move {
                    log::info!("tx_close {}", params.txid);

                    call_mod_througth_bus!(
                        bus_addr,
                        "tx_conn",
                        "connect",
                        json!(CloseConnectRequest { uid: params.uid, txid: params.txid })
                    );

                    Ok(Value::Null)
                })
            }
            _ => Box::pin(async move { Ok(Value::Null) }),
        }
    }
}

impl Handler<Event> for TransactionModule {
    type Result = ResponseFuture<Result<(), EwfError>>;
    fn handle(&mut self, msg: Event, _ctx: &mut Context<Self>) -> Self::Result {
        let event: &str = &msg.event;
        match event {
            // no care this event, ignore
            _ => Box::pin(async move { Ok(()) }),
        }
    }
}

impl Handler<StartNotify> for TransactionModule {
    type Result = ();
    fn handle(&mut self, msg: StartNotify, _ctx: &mut Context<Self>) -> Self::Result {
        self.bus_addr = Some(msg.addr);

        fn close_check_task(_self: &mut TransactionModule, ctx: &mut Context<TransactionModule>) {
            ctx.notify(Call {
                method: "run_close_check_task".to_string(),
                args: Value::Null,
            });
        }

        // 启动定时器关闭死链接
        _ctx.run_interval(Duration::new(CHECK_CLOSE_INTERVAL, 0), close_check_task);
    }
}

impl fmt::Debug for TransactionModule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("{{ {} {} }}", self.name(), self.version()))
    }
}

impl Module for TransactionModule {
    fn name(&self) -> String {
        "transaction".to_string()
    }

    fn version(&self) -> String {
        "0.1".to_string()
    }
}
