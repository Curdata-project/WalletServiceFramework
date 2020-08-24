mod error;
mod transaction_msg;
mod tx_algorithm;

mod tx_payload_mgr;

use std::fmt;

use crate::error::Error;
use crate::transaction_msg::{
    CurrencyPlan, CurrencyStat, TXMsgPackageData, TransactionConfirm, TransactionContextAck,
    TransactionContextSyn, TransactionSyn,
};
use crate::tx_payload_mgr::{
    MemFnTXPayloadGet, MemFnTXPayloadGetBySmid, MemFnTXPayloadMgrClose, MemFnTXPayloadMgrCreate,
    MemFnTXPayloadMgrCreateResult, MemFnTXSetCurrencyPlan, MemFnTXSetPayCurrencyStat,
    MemFnTXSetPaymentPlan, PeerCurrencyPlan, TXPayloadMgr, TransactionPayload,
};
use actix::prelude::*;
use chrono::prelude::Local;
use common_structure::digital_currency::DigitalCurrencyWrapper;
use common_structure::get_rng_core;
use common_structure::transaction::{Transaction, TransactionWrapper};
use ewf_core::error::Error as EwfError;
use ewf_core::states::TransactionMachine;
use ewf_core::{
    async_parse_check, async_parse_check_withlog, call_mod_througth_bus, call_self, transition,
};
use kv_object::kv_object::MsgType;
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
use wallet_common::currencies::{CurrencyStatisticsItem, QueryCurrencyStatisticsParam, CurrencyEntity, PickSpecifiedNumCurrencyParam};
use wallet_common::prepare::{ModInitialParam, ModStatus, ModStatusPullParam};
use wallet_common::transaction::CurrencyPlanItem;
use wallet_common::transaction::{
    TXCloseRequest, TXSendRequest, TXSendResponse, TransactionExchangerItem,
};
use wallet_common::secret::{SignTransactionRequest, SignTransactionResponse};
use wallet_common::user::UserEntity;
use chrono::prelude::Local;

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
                            oppo_uid: params.oppo_peer_uid.clone(),
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
                "ComputePlan" => {
                    if compute_plan(tx_payload_addr, bus_addr.clone(), payload).await? {
                        Ok(transition!(
                            bus_addr,
                            tx_sm_id,
                            "RecvComputePlanNeedExchange"
                        ))
                    } else {
                        Ok(transition!(
                            bus_addr,
                            tx_sm_id,
                            "RecvComputePlanNotNeedExchange"
                        ))
                    }
                }
                "ExchangeCurrencyAtReceiver" => {
                    // exchange_currency(tx_payload_addr.clone(), bus_addr.clone(), payload.clone()).await?;

                    // if compute_plan(tx_payload_addr, bus_addr.clone(), payload).await? {
                    //     log::error!("exchange currency but still not found avail currency plan");
                    //     Err(Error::TXExpectError.to_ewf_error())
                    // }
                    // else{
                    //     Ok(transition!(bus_addr, tx_sm_id, "ExChangeCurrencyDoneAtReceiver"))
                    // }
                    Ok(())
                }
                "SendCurrencyPlan" => {
                    send_currency_plan(tx_payload_addr, bus_addr.clone(), payload).await?;

                    Ok(transition!(bus_addr, tx_sm_id, "SendCurrencyPlanData"))
                }
                "CurrencyPlanDone" => {
                    send_transaction_syn(tx_payload_addr, bus_addr.clone(), payload).await?;

                    Ok(())
                }
                "EndTransaction" => {
                    end_transaction(tx_payload_addr, bus_addr.clone(), payload).await?;

                    Ok(())
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
                "CurrencyStat" => {
                    recv_currencystat(tx_payload_addr, bus_addr.clone(), params.msg.data, payload)
                        .await?;
                    Ok(transition!(bus_addr, tx_sm_id, "RecvCurrencyStat"))
                }
                "CurrencyPlan" => {
                    if recv_currency_plan(
                        tx_payload_addr,
                        bus_addr.clone(),
                        params.msg.data,
                        payload,
                    )
                    .await?
                    {
                        Ok(transition!(
                            bus_addr,
                            tx_sm_id,
                            "RecvCurrencyPlanNeedExchange"
                        ))
                    } else {
                        Ok(transition!(
                            bus_addr,
                            tx_sm_id,
                            "RecvCurrencyPlanNotNeedExchange"
                        ))
                    }
                }
                "TransactionSyn" => {
                    recv_transaction_syn(
                        tx_payload_addr.clone(),
                        bus_addr.clone(),
                        params.msg.data,
                        payload.clone(),
                    )
                    .await?;

                    send_transaction_confirm(tx_payload_addr, bus_addr.clone(), payload).await?;

                    Ok(transition!(bus_addr, tx_sm_id, "SendTransactionSync"))
                }
                "TransactionConfirm" => {
                    recv_transaction_confirm(
                        tx_payload_addr,
                        bus_addr.clone(),
                        params.msg.data,
                        payload,
                    )
                    .await?;

                    Ok(transition!(
                        bus_addr,
                        tx_sm_id,
                        "SendAndRecvTransactionConfirm"
                    ))
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
    };

    let mut iter = recv.exchangers.iter();
    // 双方交易流程中，另一个人就是对手交易方
    let oppo_exchanger = loop {
        if let Some(exchanger) = iter.next() {
            if exchanger.uid != payload.uid {
                break exchanger.clone();
            }
        } else {
            return Err(Error::TXCurrencyPlanNotValid.to_ewf_error());
        }
    };

    // 存储对方user信息
    call_mod_througth_bus!(
        bus_addr,
        "user",
        "add_user",
        json!(UserEntity {
            uid: oppo_exchanger.uid.clone(),
            cert: oppo_exchanger.cert,
            last_tx_time: Local::now().timestamp_millis(),
            account: oppo_exchanger.account,
        })
    );

    tx_payload_addr
        .send(MemFnTXSetPaymentPlan {
            txid: payload.txid.clone(),
            uid: payload.uid.clone(),
            oppo_uid: oppo_exchanger.uid,
            exchangers: recv.exchangers,
        })
        .await?
        .map_err(|err| err.to_ewf_error())?;
    Ok(())
}

///
/// 回应transaction_context_syn
///     在transaction_context_ack中exchangers添加自己的信息
async fn send_paymentplanack(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    payload: TransactionPayload,
) -> Result<(), EwfError> {
    
    payload.exchangers.push(TransactionExchangerItem{
        uid: String,
        cert: String,
        account: String,
        output: u64,
        intput: u64,
        addition: Value::Null,
    });
    let transaction_context_ack = TransactionContextAck {exchangers: payload.exchangers};

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
    // 存储对方user信息
    call_mod_througth_bus!(
        bus_addr,
        "user",
        "add_user",
        json!(UserEntity {
            uid: oppo_exchanger.uid.clone(),
            cert: oppo_exchanger.cert,
            last_tx_time: Local::now().timestamp_millis(),
            account: oppo_exchanger.account,
        })
    );

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
        json!(QueryCurrencyStatisticsParam{
            has_avail: true,
            has_lock: false,
            has_wait_confirm: false,
            owner_uid: payload.uid.clone(),
        })
    ), Error::TXExpectError.to_ewf_error(), log::error!("parse func currencies.query_currency_statistics return value error"));

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

async fn recv_currencystat(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    msg_data: Value,
    payload: TransactionPayload,
) -> Result<(), EwfError> {
    let recv = CurrencyStat::from_msgpack(msg_data).map_err(|err| err.to_ewf_error())?;

    tx_payload_addr
        .send(MemFnTXSetPayCurrencyStat {
            tx_sm_id: payload.tx_sm_id,
            currency_stat: recv,
        })
        .await?
        .map_err(|err| err.to_ewf_error())?;

    Ok(())
}

/// 收款者计算支付方案，返回方案是否需要找零
async fn compute_plan(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    payload: TransactionPayload,
) -> Result<bool, EwfError> {
    let statistics:Vec<CurrencyStatisticsItem> = async_parse_check_withlog!(call_mod_througth_bus!(
        bus_addr,
        "currencies",
        "query_currency_statistics",
        json!(QueryCurrencyStatisticsParam{
            has_avail: true,
            has_lock: false,
            has_wait_confirm: false,
            owner_uid: payload.uid.clone(),
        })
    ), Error::TXExpectError.to_ewf_error(), log::error!("parse func currencies.query_currency_statistics return value error"));
    let receiver_stat = tx_algorithm::get_currenics_for_change(statistics);

    let aim_amount = payload.amount % 10000u64;
    let pay_currency_stat = match payload.pay_currency_stat {
        Some(pay_currency_stat) => pay_currency_stat,
        None => {
            log::error!("exchange currency but still not found avail currency plan");
            return Err(Error::TXExpectError.to_ewf_error());
        }
    };
    log::debug!(
        "compute {:?} {:?}",
        pay_currency_stat.statistics,
        receiver_stat
    );
    let currency_plan = match tx_algorithm::find_currency_plan(
        pay_currency_stat.statistics,
        receiver_stat,
        aim_amount,
    ) {
        Ok(currency_plan) => currency_plan,
        Err(Error::TXPayNotAvailChangePlan) => return Ok(true),
        // 余额不足
        Err(err) => return Err(err.to_ewf_error()),
    };

    log::debug!("compute => {:?}", currency_plan);

    tx_payload_addr
        .send(MemFnTXSetCurrencyPlan {
            tx_sm_id: payload.tx_sm_id,
            // 仅双方，所以数组固定有两个元素
            peer_plan: vec![
                PeerCurrencyPlan {
                    uid: payload.oppo_uid,
                    item: currency_plan.clone(),
                },
                PeerCurrencyPlan {
                    uid: payload.uid,
                    item: CurrencyPlanItem {
                        pay_amount: currency_plan.recv_amount,
                        pay_plan: currency_plan.recv_plan,
                        recv_amount: currency_plan.pay_amount,
                        recv_plan: currency_plan.pay_plan,
                    },
                },
            ],
        })
        .await?
        .map_err(|err| err.to_ewf_error())?;

    if currency_plan.recv_amount != 0 {
        return Ok(true);
    }
    Ok(false)
}

async fn exchange_currency(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    payload: TransactionPayload,
) -> Result<(), EwfError> {
    Ok(())
}

async fn send_currency_plan(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    payload: TransactionPayload,
) -> Result<(), EwfError> {
    call_mod_througth_bus!(
        bus_addr,
        "tx_conn",
        "send_tx_msg",
        json!(SendMsgPackage {
            msg: MsgPackage {
                txid: payload.txid,
                data: CurrencyPlan {
                    peer_plans: payload.currency_plan
                }
                .to_msgpack(),
            },
            send_uid: payload.uid,
        })
    );

    Ok(())
}

async fn recv_currency_plan(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    msg_data: Value,
    payload: TransactionPayload,
) -> Result<bool, EwfError> {
    let recv = CurrencyPlan::from_msgpack(msg_data).map_err(|err| err.to_ewf_error())?;

    let mut iter = recv.peer_plans.iter();
    let user_plan = loop {
        if let Some(peer_plan) = iter.next() {
            if peer_plan.uid == payload.uid {
                break peer_plan.item.clone();
            }
        } else {
            return Err(Error::TXCurrencyPlanNotValid.to_ewf_error());
        }
    };

    tx_payload_addr
        .send(MemFnTXSetCurrencyPlan {
            tx_sm_id: payload.tx_sm_id,
            peer_plan: recv.peer_plans,
        })
        .await?
        .map_err(|err| err.to_ewf_error())?;

    // 按照收款方指定的支付方案，看是否需要兑零
    // TODO 支付方兑零
    // user_plan

    Ok(false)
}

async fn send_transaction_syn(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    payload: TransactionPayload,
) -> Result<(), EwfError> {

    let mut iter = payload.currency_plan.iter();
    let user_plan = loop {
        if let Some(peer_plan) = iter.next() {
            if peer_plan.uid == payload.uid {
                break peer_plan.item.clone();
            }
        } else {
            return Err(Error::TXCurrencyPlanNotValid.to_ewf_error());
        }
    };

    let picked_currencys: Vec<CurrencyEntity> = async_parse_check_withlog!(call_mod_througth_bus!(
        bus_addr,
        "currencies",
        "pick_specified_num_currency",
        json!(PickSpecifiedNumCurrencyParam{
            items: user_plan.pay_plan,
            owner_uid: payload.uid.clone(),
        })
    ), Error::TXExpectError.to_ewf_error(), log::error!("parse func currencies.pick_specified_num_currency return value error"));

    let mut sign_currencys = Vec::<DigitalCurrencyWrapper>::new();
    for each in picked_currencys{
        let currency = match each{
            CurrencyEntity::AvailEntity{id: _,
                owner_uid: _,
                value: _,
                currency,
                currency_str: _,
                txid: _,
                update_time: _,
                last_owner_id: _,} => { currency },
            _ => {
                log::error!("currencies return currency not avail type");
                return Err(Error::TXExpectError.to_ewf_error())
            }
        };
        sign_currencys.push(currency);
    }

    let signed_transactions: SignTransactionResponse = async_parse_check_withlog!(call_mod_througth_bus!(
        bus_addr,
        "secret",
        "sign_transaction",
        json!(SignTransactionRequest{
            uid: payload.uid.clone(),
            oppo_cert: ,
            datas: sign_currencys,
        })
    ), Error::TXExpectError.to_ewf_error(), log::error!("parse func currencies.pick_specified_num_currency return value error"));

    call_mod_througth_bus!(
        bus_addr,
        "tx_conn",
        "send_tx_msg",
        json!(SendMsgPackage {
            msg: MsgPackage {
                txid: payload.txid,
                data: TransactionSyn { tx_datas: signed_transactions.datas }.to_msgpack(),
            },
            send_uid: payload.uid,
        })
    );
    Ok(())
}

async fn recv_transaction_syn(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    msg_data: Value,
    payload: TransactionPayload,
) -> Result<(), EwfError> {
    Ok(())
}

async fn send_transaction_confirm(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    payload: TransactionPayload,
) -> Result<(), EwfError> {
    call_mod_througth_bus!(
        bus_addr,
        "tx_conn",
        "send_tx_msg",
        json!(SendMsgPackage {
            msg: MsgPackage {
                txid: payload.txid,
                data: TransactionConfirm {}.to_msgpack(),
            },
            send_uid: payload.uid,
        })
    );
    Ok(())
}

async fn recv_transaction_confirm(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    msg_data: Value,
    payload: TransactionPayload,
) -> Result<(), EwfError> {
    Ok(())
}

async fn end_transaction(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    payload: TransactionPayload,
) -> Result<(), EwfError> {
    Ok(())
}
