mod conn_mgr;

mod error;

use serde_json::Value;
use std::fmt;

use actix::prelude::*;
use ewf_core::error::Error as EwfError;
use ewf_core::{async_parse_check, call_mod_througth_bus, call_self};
use ewf_core::{Bus, Call, Event, Module, StartNotify};
use serde_json::json;
use wallet_common::connect::{
    BindTransPortParam, CloseBindTransPortParam, CloseConnectRequest, ConnectRequest,
    RecvMsgPackage, SendMsgPackage,
};
use wallet_common::prepare::{ModInitialParam, ModStatus, ModStatusPullParam};
use wallet_common::query::QueryParam;
use wallet_common::secret::SecretEntity;

// peer_code: u64和peer_addr: Value实际上是一致的，peer_addr为对外统一抽象，三方传递信息
pub struct TXConnModule {
    bus_addr: Option<Addr<Bus>>,

    conn_mgr_addr: Option<Addr<conn_mgr::ConnMgr>>,

    /// 启动优先级
    priority: i32,
}

impl TXConnModule {
    pub fn new() -> Self {
        Self {
            bus_addr: None,
            conn_mgr_addr: None,
            priority: 0,
        }
    }
}

impl Actor for TXConnModule {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {}
}

impl Handler<Call> for TXConnModule {
    type Result = ResponseFuture<Result<Value, EwfError>>;
    fn handle(&mut self, msg: Call, ctx: &mut Context<Self>) -> Self::Result {
        let conn_mgr_addr = self.conn_mgr_addr.clone().unwrap();
        let self_addr = ctx.address();
        let mod_name = self.name();
        let bus_addr = self.bus_addr.clone().unwrap();
        let priority = self.priority;

        Box::pin(async move {
            let method: &str = &msg.method;
            match method {
                "mod_initial" => {
                    let params: ModInitialParam =
                        async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                    if params.priority != priority {
                        return Ok(json!(ModStatus::Ignore));
                    }

                    for page_i in 1.. {
                        let data = call_mod_througth_bus!(
                            bus_addr,
                            "secret",
                            "query_secret_comb",
                            json!(QueryParam {
                                page_items: 10,
                                page_num: page_i,
                                order_by: "uid".to_string(),
                                is_asc_order: true,
                            })
                        );
                        let secrets: Vec<SecretEntity> = serde_json::from_value(data).unwrap();

                        if secrets.len() == 0 {
                            break;
                        }

                        for secret in secrets {
                            call_self!(
                                self_addr,
                                "bind_listen",
                                json!(BindTransPortParam { uid: secret.uid })
                            );
                        }
                    }

                    call_mod_througth_bus!(
                        bus_addr,
                        "prepare",
                        "mod_initial_return",
                        json!(ModStatusPullParam {
                            mod_name: mod_name,
                            is_prepare: ModStatus::InitalSuccess,
                        })
                    );

                    Ok(json!(ModStatus::InitalSuccess))
                }
                "bind_listen" => {
                    let params: BindTransPortParam =
                        async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                    conn_mgr_addr
                        .send(conn_mgr::MemFnBindListenParam { uid: params.uid })
                        .await?;

                    Ok(Value::Null)
                }
                "close_bind" => {
                    let params: CloseBindTransPortParam =
                        async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                    conn_mgr_addr
                        .send(conn_mgr::MemFnCloseBindParam { uid: params.uid })
                        .await?;

                    Ok(Value::Null)
                }
                "connect" => {
                    let params: ConnectRequest =
                        async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                    conn_mgr_addr
                        .send(conn_mgr::MemFnConnectParam {
                            self_uid: params.uid,
                            peer_uid: params.oppo_peer_uid,
                            txid: params.txid,
                        })
                        .await?
                        .map_err(|err| err.to_ewf_error())?;

                    Ok(Value::Null)
                }
                "close_conn" => {
                    let params: CloseConnectRequest =
                        async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                    conn_mgr_addr
                        .send(conn_mgr::MemFnCloseParam { txid: params.txid })
                        .await?;

                    Ok(Value::Null)
                }
                "send_tx_msg" => {
                    let params: SendMsgPackage =
                        async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                    conn_mgr_addr
                        .send(conn_mgr::MemFnSendParam {
                            send_uid: params.send_uid,
                            txid: params.msg.txid,
                            data: params.msg.data,
                        })
                        .await?
                        .map_err(|err| err.to_ewf_error())?;

                    Ok(Value::Null)
                }
                "recv_tx_msg" => {
                    let params: RecvMsgPackage =
                        async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                    log::debug!(
                        "RECV: UID {} TX {} => DATA {}",
                        params.recv_uid,
                        params.msg.txid,
                        params.msg.data
                    );

                    Ok(Value::Null)
                }
                _ => Ok(Value::Null),
            }
        })
    }
}

impl Handler<Event> for TXConnModule {
    type Result = ResponseFuture<Result<(), EwfError>>;
    fn handle(&mut self, _msg: Event, _ctx: &mut Context<Self>) -> Self::Result {
        Box::pin(async move {
            let event: &str = &_msg.event;
            match event {
                // no care this event, ignore
                _ => return Ok(()),
            }
        })
    }
}

impl Handler<StartNotify> for TXConnModule {
    type Result = ();
    fn handle(&mut self, msg: StartNotify, ctx: &mut Context<Self>) -> Self::Result {
        self.bus_addr = Some(msg.addr.clone());
        self.priority = msg.priority;

        self.conn_mgr_addr = Some(conn_mgr::ConnMgr::new(ctx.address()).start());
    }
}

impl fmt::Debug for TXConnModule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("{{ {} {} }}", self.name(), self.version()))
    }
}

impl Module for TXConnModule {
    fn name(&self) -> String {
        "tx_conn".to_string()
    }

    fn version(&self) -> String {
        "0.1".to_string()
    }
}
