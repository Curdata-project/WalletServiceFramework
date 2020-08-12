mod conn_mgr;

mod error;

use serde_json::Value;
use std::fmt;

use actix::prelude::*;
use ewf_core::async_parse_check;
use ewf_core::error::Error as EwfError;
use ewf_core::{Bus, Call, CallQuery, Event, Module, StartNotify};
use serde_json::json;
use wallet_common::connect::*;
use wallet_common::prepare::{ModStatus, ModStatusPullParam};

// peer_code: u64和peer_addr: Value实际上是一致的，peer_addr为对外统一抽象，三方传递信息
pub struct TXConnModule {
    bus_addr: Option<Addr<Bus>>,

    conn_mgr_addr: Option<Addr<conn_mgr::ConnMgr>>,
}

impl TXConnModule {
    pub fn new() -> Self {
        Self {
            bus_addr: None,
            conn_mgr_addr: None,
        }
    }
}

impl Actor for TXConnModule {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {}
}

impl Handler<Call> for TXConnModule {
    type Result = ResponseFuture<Result<Value, EwfError>>;
    fn handle(&mut self, msg: Call, _ctx: &mut Context<Self>) -> Self::Result {
        let conn_mgr_addr = self.conn_mgr_addr.clone().unwrap();

        let method: &str = &msg.method;
        match method {
            "bind_listen" => Box::pin(async move {
                let params: BindTransPortParam =
                    async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                conn_mgr_addr
                    .send(conn_mgr::MemFnBindListenParam { uid: params.uid })
                    .await?;

                Ok(Value::Null)
            }),
            "close_bind" => Box::pin(async move {
                let params: CloseBindTransPortParam =
                    async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                conn_mgr_addr
                    .send(conn_mgr::MemFnCloseBindParam { uid: params.uid })
                    .await?;

                Ok(Value::Null)
            }),
            "connect" => Box::pin(async move {
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
            }),
            "close_conn" => Box::pin(async move {
                let params: CloseConnectRequest =
                    async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                conn_mgr_addr
                    .send(conn_mgr::MemFnCloseParam { txid: params.txid })
                    .await?;

                Ok(Value::Null)
            }),
            "send_tx_msg" => Box::pin(async move {
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
            }),
            "recv_tx_msg" => Box::pin(async move {
                let params: RecvMsgPackage =
                    async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                log::debug!(
                    "RECV: UID {} TX {} => DATA {}",
                    params.recv_uid,
                    params.msg.txid,
                    params.msg.data
                );

                Ok(Value::Null)
            }),
            _ => Box::pin(async move { Ok(Value::Null) }),
        }
    }
}

impl Handler<Event> for TXConnModule {
    type Result = ResponseFuture<Result<(), EwfError>>;
    fn handle(&mut self, msg: Event, _ctx: &mut Context<Self>) -> Self::Result {
        let bus_addr = self.bus_addr.clone().unwrap();
        let mod_name = self.name();

        let event: &str = &msg.event;
        match event {
            "Start" => Box::pin(async move {
                let prepare = bus_addr
                    .send(CallQuery {
                        module: "prepare".to_string(),
                    })
                    .await??;
                prepare
                    .send(Call {
                        method: "inital".to_string(),
                        args: json!(ModStatusPullParam {
                            mod_name: mod_name,
                            is_prepare: ModStatus::InitalSuccess,
                        }),
                    })
                    .await??;

                Ok(())
            }),
            // no care this event, ignore
            _ => Box::pin(async move { Ok(()) }),
        }
    }
}

impl Handler<StartNotify> for TXConnModule {
    type Result = ();
    fn handle(&mut self, msg: StartNotify, ctx: &mut Context<Self>) -> Self::Result {
        self.bus_addr = Some(msg.addr.clone());
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
