mod error;
mod transaction_msg;
mod tx_algorithm;

mod tx_payload_mgr;

use std::fmt;

use crate::error::Error;
use crate::transaction_msg::{
    CurrencyStat, TXMsgPackageData, TransactionContextAck, TransactionContextSyn,
};
use crate::tx_payload_mgr::{
    MemFnTXPayloadGet, MemFnTXPayloadGetBySmid, MemFnTXPayloadMgrClose, MemFnTXPayloadMgrCreate,
    MemFnTXPayloadMgrCreateResult, MemFnTXSetPaymentPlan, TXPayloadMgr, TransactionPayload,
};
use actix::prelude::*;
use chrono::prelude::Local;
use common_structure::digital_currency::DigitalCurrencyWrapper;
use common_structure::get_rng_core;
use ewf_core::error::Error as EwfError;
use ewf_core::states::TransactionMachine;
use ewf_core::{
    async_parse_check, async_parse_check_withlog, call_mod_througth_bus, call_self, transition,
};
use ewf_core::{Bus, Call, CreateMachine, Event, Module, StartNotify};
use hex::ToHex;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::hash_map::HashMap;
use std::time::Duration;
use wallet_common::connect::{
    CloseConnectRequest, ConnectRequest, MsgPackage, OnConnectNotify, RecvMsgPackage,
    SendMsgPackage,
};
use wallet_common::currencies::CurrencyStatisticsItem;
use wallet_common::prepare::{ModInitialParam, ModStatus, ModStatusPullParam};
use wallet_common::transaction::{
    TXCloseRequest, TXSendRequest, TXSendResponse, TransactionExchangerItem,
};

/// 交易时钟最大允许偏差 ms
const MAX_TRANSACTION_CLOCK_SKEW_MS: i64 = 30000;

#[derive(Debug, Message, Clone, Serialize, Deserialize)]
#[rtype(result = "Result<(), EwfError>")]
struct RecvMsgPackageByTxConn {
    msg: MsgPackage,
    recv_uid: String,
}

pub struct TransactionModule {
    bus_addr: Option<Addr<Bus>>,
    tx_payload_addr: Option<Addr<TXPayloadMgr>>,
}

impl TransactionModule {
    pub fn new() -> Self {
        Self {
            bus_addr: None,
            tx_payload_addr: None,
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
        let self_addr = ctx.address();
        let bus_addr = self.bus_addr.clone().unwrap();
        let tx_payload_addr = self.tx_payload_addr.clone().unwrap();

        Box::pin(async move {
            let method: &str = &msg.method;
            match method {
                "mod_initial" => {
                    let params: ModInitialParam =
                        async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                    Ok(json!(ModStatus::InitalSuccess))
                }
                "on_connect" => {
                    let params: OnConnectNotify =
                        async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                    // 创建状态机
                    let tx_sm_id = bus_addr
                        .send(CreateMachine {
                            machine: Box::new(TransactionMachine::default()),
                        })
                        .await?;

                    let save_ans: MemFnTXPayloadMgrCreateResult = tx_payload_addr
                        .send(MemFnTXPayloadMgrCreate {
                            uid: params.uid.clone(),
                            tx_sm_id,
                            is_tx_sender: false,
                            txid: Some(params.txid),
                        })
                        .await?
                        .map_err(|err| err.to_ewf_error())?;

                    log::info!("tx_on_connect {} at {}", save_ans.txid.clone(), params.uid);

                    transition!(bus_addr, tx_sm_id, "Starting");

                    Ok(json!(TXSendResponse {
                        txid: save_ans.txid,
                    }))
                }
                "tx_send" => {
                    let params: TXSendRequest =
                        async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                    // 创建状态机
                    let tx_sm_id = bus_addr
                        .send(CreateMachine {
                            machine: Box::new(TransactionMachine::default()),
                        })
                        .await?;

                    let save_ans: MemFnTXPayloadMgrCreateResult = tx_payload_addr
                        .send(MemFnTXPayloadMgrCreate {
                            uid: params.uid.clone(),
                            tx_sm_id,
                            is_tx_sender: true,
                            txid: None,
                        })
                        .await?
                        .map_err(|err| err.to_ewf_error())?;

                    tx_payload_addr
                        .send(MemFnTXSetPaymentPlan {
                            txid: save_ans.txid.clone(),
                            uid: params.uid.clone(),
                            exchangers: params.exchangers,
                        })
                        .await?
                        .map_err(|err| err.to_ewf_error())?;

                    // 建立链接，若此处连接失败，创建的状态机等随超时回收
                    call_mod_througth_bus!(
                        bus_addr,
                        "tx_conn",
                        "connect",
                        json!(ConnectRequest {
                            uid: params.uid.clone(),
                            oppo_peer_uid: params.oppo_peer_uid,
                            txid: save_ans.txid.clone(),
                        })
                    );
                    log::info!("tx_connect {} at {}", save_ans.txid.clone(), params.uid);

                    transition!(bus_addr, tx_sm_id, "Starting");

                    Ok(json!(TXSendResponse {
                        txid: save_ans.txid,
                    }))
                }
                "recv_tx_msg" => {
                    let params: RecvMsgPackage =
                        async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                    let uid = params.recv_uid.clone();
                    let txid = params.msg.txid.clone();

                    match self_addr
                        .send(RecvMsgPackageByTxConn {
                            msg: params.msg,
                            recv_uid: params.recv_uid,
                        })
                        .await
                    {
                        Ok(Ok(_)) => {}
                        Ok(Err(err)) => {
                            call_self!(
                                self_addr,
                                "tx_close",
                                json!(TXCloseRequest {
                                    txid,
                                    uid,
                                    reason: format!("{:?}", err)
                                })
                            );
                        }
                        Err(err) => {
                            call_self!(
                                self_addr,
                                "tx_close",
                                json!(TXCloseRequest {
                                    txid,
                                    uid,
                                    reason: format!("{:?}", err)
                                })
                            );
                        }
                    }

                    Ok(Value::Null)
                }
                "tx_close" => {
                    let params: TXCloseRequest =
                        async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                    tx_payload_addr
                        .send(MemFnTXPayloadMgrClose {
                            txid: params.txid.clone(),
                            uid: params.uid.clone(),
                        })
                        .await?;

                    log::info!(
                        "tx_close {} at {}, reason: {}",
                        params.txid,
                        params.uid,
                        params.reason
                    );

                    call_mod_througth_bus!(
                        bus_addr,
                        "tx_conn",
                        "close_conn",
                        json!(CloseConnectRequest { txid: params.txid })
                    );

                    Ok(Value::Null)
                }
                _ => Ok(Value::Null),
            }
        })
    }
}

impl Handler<Event> for TransactionModule {
    type Result = ResponseFuture<Result<(), EwfError>>;
    fn handle(&mut self, msg: Event, _ctx: &mut Context<Self>) -> Self::Result {
        let bus_addr = self.bus_addr.clone().unwrap();
        let tx_payload_addr = self.tx_payload_addr.clone().unwrap();

        Box::pin(async move {
            let event: &str = &msg.event;
            let tx_sm_id = msg.id;

            let payload: TransactionPayload = tx_payload_addr
                .send(MemFnTXPayloadGetBySmid {
                    tx_sm_id: tx_sm_id.clone(),
                })
                .await?
                .map_err(|err| err.to_ewf_error())?;

            match event {
                "Start" => {
                    // Start状态时，exchangers不为空的为交易发起方
                    if payload.exchangers.len() != 0usize {
                        send_transaction_context_syn(tx_payload_addr, bus_addr.clone(), payload)
                            .await?;

                        return Ok(transition!(bus_addr, tx_sm_id, "PaymentPlanSyn"));
                    }
                    Ok(())
                }
                "PaymentPlanRecv" => {
                    send_paymentplanack(tx_payload_addr, bus_addr.clone(), payload).await?;

                    Ok(transition!(bus_addr, tx_sm_id, "PaymentPlanAck"))
                }
                "PaymentPlanSend" => Ok(()),
                "PaymentPlanDone" => {
                    if payload.is_payer {
                        send_currency_stat(tx_payload_addr, bus_addr.clone(), payload).await?;

                        Ok(transition!(
                            bus_addr,
                            tx_sm_id,
                            "IsPayerAndSendCurrencyStat"
                        ))
                    } else {
                        Ok(transition!(bus_addr, tx_sm_id, "IsReceiver"))
                    }
                }
                // no care this event, ignore
                _ => Ok(()),
            }
        })
    }
}

impl Handler<StartNotify> for TransactionModule {
    type Result = ();
    fn handle(&mut self, msg: StartNotify, _ctx: &mut Context<Self>) -> Self::Result {
        self.bus_addr = Some(msg.addr);

        self.tx_payload_addr = Some(TXPayloadMgr::new(_ctx.address()).start());
    }
}

impl Handler<RecvMsgPackageByTxConn> for TransactionModule {
    type Result = ResponseFuture<Result<(), EwfError>>;
    fn handle(&mut self, params: RecvMsgPackageByTxConn, _ctx: &mut Context<Self>) -> Self::Result {
        let bus_addr = self.bus_addr.clone().unwrap();
        let tx_payload_addr = self.tx_payload_addr.clone().unwrap();

        Box::pin(async move {
            let tx_msgtype: &str = &transaction_msg::get_msgtype(&params.msg.data);

            let payload = tx_payload_addr
                .send(MemFnTXPayloadGet {
                    txid: params.msg.txid.clone(),
                    uid: params.recv_uid.clone(),
                })
                .await?
                .map_err(|err| err.to_ewf_error())?;

            let tx_sm_id = payload.tx_sm_id.clone();

            match tx_msgtype {
                "TransactionContextSyn" => {
                    recv_paymentplansyn(
                        tx_payload_addr,
                        bus_addr.clone(),
                        params.msg.data,
                        payload,
                    )
                    .await?;

                    Ok(transition!(bus_addr, tx_sm_id, "RecvPaymentPlanSyn"))
                }
                "TransactionContextAck" => {
                    recv_paymentplanack(
                        tx_payload_addr,
                        bus_addr.clone(),
                        params.msg.data,
                        payload,
                    )
                    .await?;

                    Ok(transition!(bus_addr, tx_sm_id, "RecvPaymentPlanAck"))
                }
                _ => {
                    log::warn!("recv unexpect tx msg <{}>", tx_msgtype,);
                    Err(Error::TXSequenceNotExpect.to_ewf_error())
                }
            }
        })
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

async fn send_transaction_context_syn(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    payload: TransactionPayload,
) -> Result<(), EwfError> {
    let transaction_context_syn = TransactionContextSyn {
        timestamp: Local::now().timestamp_millis(),
        exchangers: payload.exchangers,
    };
    call_mod_througth_bus!(
        bus_addr,
        "tx_conn",
        "send_tx_msg",
        json!(SendMsgPackage {
            send_uid: payload.uid.clone(),
            msg: MsgPackage {
                txid: payload.txid,
                data: transaction_context_syn.to_msgpack()
            }
        })
    );

    Ok(())
}

async fn recv_paymentplansyn(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    msg_data: Value,
    payload: TransactionPayload,
) -> Result<(), EwfError> {
    let recv = TransactionContextSyn::from_msgpack(msg_data).map_err(|err| err.to_ewf_error())?;

    let now = Local::now().timestamp_millis();
    if now - recv.timestamp > MAX_TRANSACTION_CLOCK_SKEW_MS {
        return Err(Error::TXCLOCKSKEWTOOLARGE.to_ewf_error());
    }

    tx_payload_addr
        .send(MemFnTXSetPaymentPlan {
            txid: payload.txid.clone(),
            uid: payload.uid.clone(),
            exchangers: recv.exchangers,
        })
        .await?
        .map_err(|err| err.to_ewf_error())?;
    Ok(())
}

async fn send_paymentplanack(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    payload: TransactionPayload,
) -> Result<(), EwfError> {
    let transaction_context_ack = TransactionContextAck {};

    call_mod_througth_bus!(
        bus_addr,
        "tx_conn",
        "send_tx_msg",
        json!(SendMsgPackage {
            msg: MsgPackage {
                txid: payload.txid,
                data: transaction_context_ack.to_msgpack(),
            },
            send_uid: payload.uid,
        })
    );

    Ok(())
}

async fn recv_paymentplanack(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    msg_data: Value,
    payload: TransactionPayload,
) -> Result<(), EwfError> {
    Ok(())
}

async fn send_currency_stat(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    payload: TransactionPayload,
) -> Result<(), EwfError> {
    let statistics:Vec<CurrencyStatisticsItem> = async_parse_check_withlog!(call_mod_througth_bus!(
        bus_addr,
        "currencies",
        "query_currency_statistics",
        json!(payload.uid)
    ), Error::TXExpectError.to_ewf_error(), log::error!("parse func currencies.query_currency_statistics return value error when deal tx_msg TransactionContextSyn"));

    let currency_stat = CurrencyStat {
        statistics: tx_algorithm::get_currenics_for_change(statistics),
    };

    call_mod_througth_bus!(
        bus_addr,
        "tx_conn",
        "send_tx_msg",
        json!(SendMsgPackage {
            msg: MsgPackage {
                txid: payload.txid,
                data: currency_stat.to_msgpack(),
            },
            send_uid: payload.uid,
        })
    );

    Ok(())
}

// async fn recv_paymentplanack_when_paymentplan_done(tx_payload_addr: Addr<TXPayloadMgr>, bus_addr: Addr<Bus>, msg_data: Value, payload: TransactionPayload) -> Result<(), EwfError> {
//     transition!(bus_addr, payload.tx_sm_id, "RecvPaymentPlanAck");

//     Ok(())
// }
