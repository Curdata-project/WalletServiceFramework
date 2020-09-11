#![feature(async_closure)]
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
    MemFnTXSetPayLockCurrencys, MemFnTXSetPaymentPlan, MemFnTXSetRecvWaitConfirmCurrencys,
    MemFnTXTransactionConfirm, PeerCurrencyPlan, TXPayloadMgr, TransactionPayload,
};
use actix::prelude::*;
use chrono::prelude::Local;
use common_structure::digital_currency::DigitalCurrencyWrapper;
use dislog_hal::Bytes;
use ewf_core::error::Error as EwfError;
use ewf_core::states::TransactionMachine;
use ewf_core::{async_parse_check, async_parse_check_withlog, call_mod_througth_bus, transition};
use ewf_core::{Bus, Call, CreateMachine, Event, Module, StartNotify};
use hex::{FromHex, ToHex};
use kv_object::sm2::CertificateSm2;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tx_algorithm::ComputeCurrencyPlan;
use wallet_common::connect::{
    CloseConnectRequest, ConnectRequest, MsgPackage, OnConnectNotify, RecvMsgPackage,
    SendMsgPackage,
};
use wallet_common::currencies::{
    AddCurrencyParam, CurrencyEntity, PickSpecifiedNumCurrencyParam, QueryCurrencyStatisticsParam,
    StatisticsItem, UnLockCurrencyParam,
};
use wallet_common::prepare::{ModInitialParam, ModStatus};
use wallet_common::secret::{SignTransactionRequest, SignTransactionResponse};
use wallet_common::transaction::CurrencyPlanItem;
use wallet_common::transaction::{
    TXCloseRequest, TXSendRequest, TXSendResponse, TransactionExchangerItem,
};
use wallet_common::user::UserEntity;
use wallet_common::WALLET_SM_CODE;
use wallet_common::connect::TransactionType;

/// 交易时钟最大允许偏差 ms
const MAX_TRANSACTION_CLOCK_SKEW_MS: i64 = 30000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TXSuccessParam {
    pub txid: String,
    pub uid: String,
}

#[derive(Debug, Message, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
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

                    let user: UserEntity = async_parse_check_withlog!(
                        call_mod_througth_bus!(
                            bus_addr,
                            "user",
                            "query_user",
                            json!(params.uid.clone())
                        ),
                        Error::TXExpectError.to_ewf_error(),
                        log::error!("parse func user.query_user return value error")
                    );
                    let self_exchanger = TransactionExchangerItem {
                        uid: params.uid.clone(),
                        cert: user.cert,
                        output: params.output,
                        input: params.input,
                        /// 预留字段
                        addition: json!({"account": user.account}),
                    };

                    tx_payload_addr
                        .send(MemFnTXSetPaymentPlan {
                            txid: save_ans.txid.clone(),
                            uid: params.uid.clone(),
                            oppo_uid: params.oppo_peer_uid.clone(),
                            self_exchanger: Some(self_exchanger),
                            other_exchangers: None,
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

                    self_addr
                        .send(RecvMsgPackageByTxConn {
                            msg: params.msg,
                            recv_uid: params.recv_uid,
                        })
                        .await?;

                    Ok(Value::Null)
                }
                "tx_success" => {
                    let params: TXSuccessParam =
                        async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                    let payload = tx_payload_addr
                        .send(MemFnTXPayloadMgrClose {
                            txid: params.txid.clone(),
                            uid: params.uid.clone(),
                        })
                        .await?;

                    if let Some(payload) = payload {
                        log::info!(
                            "tx_success {} at {}, is_payer {}, amount {}",
                            params.txid,
                            params.uid,
                            payload.is_payer,
                            payload.amount
                        );
                    }

                    call_mod_througth_bus!(
                        bus_addr,
                        "tx_conn",
                        "close_conn",
                        json!(CloseConnectRequest { uid: params.uid, txid: params.txid })
                    );

                    Ok(Value::Null)
                }
                "tx_close" => {
                    let params: TXCloseRequest =
                        async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                    let payload = tx_payload_addr
                        .send(MemFnTXPayloadMgrClose {
                            txid: params.txid.clone(),
                            uid: params.uid.clone(),
                        })
                        .await?;

                    if let Some(payload) = payload {
                        log::info!("unlock pay lock currency {:?}", payload.pay_lock_currencys);
                        // 解锁因交易锁定的货币
                        call_mod_througth_bus!(
                            bus_addr,
                            "currencies",
                            "unlock_currency",
                            json!(UnLockCurrencyParam {
                                ids: payload.pay_lock_currencys
                            })
                        );
                    }

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
                        json!(CloseConnectRequest { uid: params.uid, txid: params.txid })
                    );

                    Ok(Value::Null)
                }
                _ => Err(EwfError::MethodNotFoundError),
            }
        })
    }
}

impl Handler<Event> for TransactionModule {
    type Result = ResponseFuture<Result<(), EwfError>>;
    fn handle(&mut self, msg: Event, ctx: &mut Context<Self>) -> Self::Result {
        let self_addr = ctx.address();
        let bus_addr = self.bus_addr.clone().unwrap();
        let tx_payload_addr = self.tx_payload_addr.clone().unwrap();
        let tx_payload_addr1 = self.tx_payload_addr.clone().unwrap();

        let dispatch_event_task = async move |payload: TransactionPayload,
                                              msg: Event|
                    -> Result<(), Error> {
            let event: &str = &msg.event;
            let tx_sm_id = msg.id;

            match event {
                "Start" => {
                    // Start状态时，self_exchanger不为空的为交易发起方
                    if !payload.self_exchanger.is_none() {
                        send_transaction_context_syn(tx_payload_addr, bus_addr.clone(), payload)
                            .await
                            .map_err(|err| {
                                log::info!("{:?}", err);
                                err
                            })?;

                        Ok(transition!(bus_addr, tx_sm_id, "PaymentPlanSyn"))
                    } else {
                        Ok(())
                    }
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
                    //     Err(Error::TXExpectError)
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
        };

        // 被Event驱动的task是do_send放入，不捕获错误，所以错误在此捕捉
        //   返回错误原因到tx_close
        //      不过如果状态机已销毁，payload无法找到，就无法找到tx_sm_id对应的(txid, uid)，此情况仅打印错误日志
        Box::pin(async move {
            let event = msg.event.clone();
            let tx_sm_id = msg.id.clone();

            // 忽略状态机0即主事件循环的消息，否则会有一条error log
            if tx_sm_id == WALLET_SM_CODE {
                return Ok(());
            }

            let payload: TransactionPayload = match tx_payload_addr1
                .send(MemFnTXPayloadGetBySmid {
                    tx_sm_id: tx_sm_id.clone(),
                })
                .await
            {
                Ok(Ok(payload)) => payload,
                Ok(Err(_)) => {
                    log::error!(
                        "dispatch_event_task => machine has destoryed {:?}",
                        tx_sm_id
                    );
                    return Ok(());
                }
                Err(_) => {
                    // 投递错误，此处没办法处理，忽略
                    return Ok(());
                }
            };
            let txid = payload.txid.clone();
            let uid = payload.uid.clone();

            match dispatch_event_task(payload, msg).await {
                Ok(_) => Ok(()),
                Err(err) => {
                    log::error!(
                        "dispatch_event_task => {:?}, machine: {:?} event: {}",
                        err,
                        tx_sm_id.clone(),
                        event
                    );

                    self_addr.do_send(ewf_core::Call {
                        method: "tx_close".to_string(),
                        args: json!(TXCloseRequest {
                            txid,
                            uid,
                            reason: err.to_string(),
                        }),
                    });
                    Ok(())
                }
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
    type Result = ResponseFuture<()>;
    fn handle(&mut self, params: RecvMsgPackageByTxConn, ctx: &mut Context<Self>) -> Self::Result {
        let self_addr = ctx.address();
        let bus_addr = self.bus_addr.clone().unwrap();
        let tx_payload_addr = self.tx_payload_addr.clone().unwrap();

        let dispatch_tx_msg_task =
            async move |params: RecvMsgPackageByTxConn| -> Result<(), Error> {
                let tx_msgtype: &str = &transaction_msg::get_msgtype(&params.msg.data);

                let payload = tx_payload_addr
                    .send(MemFnTXPayloadGet {
                        txid: params.msg.txid.clone(),
                        uid: params.recv_uid.clone(),
                    })
                    .await??;

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
                        recv_currencystat(
                            tx_payload_addr,
                            bus_addr.clone(),
                            params.msg.data,
                            payload,
                        )
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

                        send_transaction_confirm(tx_payload_addr, bus_addr.clone(), payload)
                            .await?;

                        Ok(transition!(
                            bus_addr,
                            tx_sm_id,
                            "RecvTransactionSyncAndSendConfirm"
                        ))
                    }
                    "TransactionConfirm" => {
                        recv_transaction_confirm(
                            tx_payload_addr,
                            bus_addr.clone(),
                            params.msg.data,
                            payload,
                        )
                        .await?;

                        Ok(transition!(bus_addr, tx_sm_id, "RecvTransactionConfirm"))
                    }
                    _ => {
                        log::warn!("recv unexpect tx msg <{}>", tx_msgtype,);
                        Err(Error::TXSequenceNotExpect)
                    }
                }
            };

        // 被tx_msg驱动的task错误在此捕捉
        //   返回错误原因到tx_close
        Box::pin(async move {
            let txid = params.msg.txid.clone();
            let uid = params.recv_uid.clone();

            let ret = dispatch_tx_msg_task(params).await;
            match ret {
                Ok(_) => {}
                Err(err) => {
                    log::error!("dispatch_tx_msg_task => {:?}", err);
                    self_addr.do_send(ewf_core::Call {
                        method: "tx_close".to_string(),
                        args: json!(TXCloseRequest {
                            txid,
                            uid,
                            reason: err.to_string(),
                        }),
                    });
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
) -> Result<(), Error> {
    let transaction_context_syn = TransactionContextSyn {
        timestamp: Local::now().timestamp_millis(),
        exchangers: vec![payload.self_exchanger.unwrap()],
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
    msg_data: TransactionType,
    payload: TransactionPayload,
) -> Result<(), Error> {
    let recv = TransactionContextSyn::from_msgpack(msg_data)?;

    let now = Local::now().timestamp_millis();
    if now - recv.timestamp > MAX_TRANSACTION_CLOCK_SKEW_MS {
        return Err(Error::TXCLOCKSKEWTOOLARGE);
    };

    // 双方交易流程中，收到的数组唯一元素就是对手方
    if recv.exchangers.len() != 1 {
        log::error!("recv_paymentplansyn exchangers info error");
    }

    let oppo_exchanger = &recv.exchangers[0];
    if oppo_exchanger.addition["account"] == Value::Null {
        log::error!("recv_paymentplansyn exchangers info error");
    }

    // 存储对方user信息
    call_mod_througth_bus!(
        bus_addr,
        "user",
        "add_user",
        json!(UserEntity {
            uid: oppo_exchanger.uid.clone(),
            cert: oppo_exchanger.cert.clone(),
            last_tx_time: Local::now().timestamp_millis(),
            account: oppo_exchanger.addition["account"]
                .as_str()
                .unwrap()
                .to_string(),
        })
    );

    let user: UserEntity = async_parse_check_withlog!(
        call_mod_througth_bus!(bus_addr, "user", "query_user", json!(payload.uid.clone())),
        Error::TXExpectError,
        log::error!("parse func user.query_user return value error")
    );

    let self_exchanger = TransactionExchangerItem {
        uid: payload.uid.clone(),
        cert: user.cert,
        output: oppo_exchanger.input,
        input: oppo_exchanger.output,
        /// 预留字段
        addition: json!({"account": user.account}),
    };

    tx_payload_addr
        .send(MemFnTXSetPaymentPlan {
            txid: payload.txid,
            uid: payload.uid,
            oppo_uid: oppo_exchanger.uid.clone(),
            self_exchanger: Some(self_exchanger),
            other_exchangers: Some(vec![oppo_exchanger.clone()]),
        })
        .await??;
    Ok(())
}

/// 回应transaction_context_syn
///     在transaction_context_ack中exchangers添加自己的信息
async fn send_paymentplanack(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    payload: TransactionPayload,
) -> Result<(), Error> {
    let transaction_context_ack = TransactionContextAck {
        exchangers: vec![payload.self_exchanger.unwrap()],
    };

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
    msg_data: TransactionType,
    payload: TransactionPayload,
) -> Result<(), Error> {
    let recv = TransactionContextAck::from_msgpack(msg_data)?;

    // 存储对方user信息
    // 双方交易流程中，收到的数组唯一元素就是对手方
    if recv.exchangers.len() != 1 {
        log::error!("recv_paymentplansyn exchangers info error");
    }

    let oppo_exchanger = &recv.exchangers[0];
    if oppo_exchanger.addition["account"] == Value::Null {
        log::error!("recv_paymentplansyn exchangers info error");
    }

    tx_payload_addr
        .send(MemFnTXSetPaymentPlan {
            txid: payload.txid.clone(),
            uid: payload.uid.clone(),
            oppo_uid: oppo_exchanger.uid.clone(),
            self_exchanger: None,
            other_exchangers: Some(vec![oppo_exchanger.clone()]),
        })
        .await??;

    // 存储对方user信息
    call_mod_througth_bus!(
        bus_addr,
        "user",
        "add_user",
        json!(UserEntity {
            uid: oppo_exchanger.uid.clone(),
            cert: oppo_exchanger.cert.clone(),
            last_tx_time: Local::now().timestamp_millis(),
            account: oppo_exchanger.addition["account"]
                .as_str()
                .unwrap()
                .to_string(),
        })
    );

    Ok(())
}

async fn send_currency_stat(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    payload: TransactionPayload,
) -> Result<(), Error> {
    let statistics: Vec<StatisticsItem> = async_parse_check_withlog!(
        call_mod_througth_bus!(
            bus_addr,
            "currencies",
            "query_currency_statistics",
            json!(QueryCurrencyStatisticsParam {
                has_avail: true,
                has_lock: false,
                has_wait_confirm: false,
                owner_uid: payload.uid.clone(),
            })
        ),
        Error::TXExpectError,
        log::error!("parse func currencies.query_currency_statistics return value error")
    );

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

async fn recv_currencystat(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    msg_data: TransactionType,
    payload: TransactionPayload,
) -> Result<(), Error> {
    let recv = CurrencyStat::from_msgpack(msg_data)?;

    tx_payload_addr
        .send(MemFnTXSetPayCurrencyStat {
            tx_sm_id: payload.tx_sm_id,
            currency_stat: recv,
        })
        .await??;

    Ok(())
}

/// 收款者计算支付方案，返回方案是否需要找零
async fn compute_plan(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    payload: TransactionPayload,
) -> Result<bool, Error> {
    let statistics: Vec<StatisticsItem> = async_parse_check_withlog!(
        call_mod_througth_bus!(
            bus_addr,
            "currencies",
            "query_currency_statistics",
            json!(QueryCurrencyStatisticsParam {
                has_avail: true,
                has_lock: false,
                has_wait_confirm: false,
                owner_uid: payload.uid.clone(),
            })
        ),
        Error::TXExpectError,
        log::error!("parse func currencies.query_currency_statistics return value error")
    );
    let receiver_stat = tx_algorithm::get_currenics_for_change(statistics);

    let aim_amount = payload.amount % 10000u64;
    let pay_currency_stat = match payload.pay_currency_stat {
        Some(pay_currency_stat) => pay_currency_stat,
        None => {
            log::error!("exchange currency but still not found avail currency plan");
            return Err(Error::TXExpectError);
        }
    };
    let currency_plan = match tx_algorithm::ComputeCurrencyPlanA::new().find_currency_plan(
        pay_currency_stat.statistics,
        receiver_stat,
        aim_amount,
    ) {
        Ok(currency_plan) => currency_plan,
        Err(Error::TXPayNotAvailChangePlan) => return Ok(true),
        // 余额不足
        Err(err) => return Err(err),
    };

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
        .await??;

    if currency_plan.recv_amount != 0 {
        return Ok(true);
    }
    Ok(false)
}

async fn exchange_currency(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    payload: TransactionPayload,
) -> Result<(), Error> {
    Ok(())
}

async fn send_currency_plan(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    payload: TransactionPayload,
) -> Result<(), Error> {
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
    msg_data: TransactionType,
    payload: TransactionPayload,
) -> Result<bool, Error> {
    let recv = CurrencyPlan::from_msgpack(msg_data)?;

    let mut iter = recv.peer_plans.iter();
    let user_plan = loop {
        if let Some(peer_plan) = iter.next() {
            if peer_plan.uid == payload.uid {
                break peer_plan.item.clone();
            }
        } else {
            return Err(Error::TXCurrencyPlanNotValid);
        }
    };

    tx_payload_addr
        .send(MemFnTXSetCurrencyPlan {
            tx_sm_id: payload.tx_sm_id,
            peer_plan: recv.peer_plans,
        })
        .await??;

    // 按照收款方指定的支付方案，看是否需要兑零
    // TODO 支付方兑零
    // user_plan

    Ok(false)
}

async fn send_transaction_syn(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    payload: TransactionPayload,
) -> Result<(), Error> {
    let mut iter = payload.currency_plan.iter();
    let user_plan = loop {
        if let Some(peer_plan) = iter.next() {
            if peer_plan.uid == payload.uid {
                break peer_plan.item.clone();
            }
        } else {
            return Err(Error::TXCurrencyPlanNotValid);
        }
    };

    let picked_currencys: Vec<CurrencyEntity> = async_parse_check_withlog!(
        call_mod_througth_bus!(
            bus_addr,
            "currencies",
            "pick_specified_num_currency",
            json!(PickSpecifiedNumCurrencyParam {
                items: user_plan.pay_plan,
                owner_uid: payload.uid.clone(),
            })
        ),
        Error::TXExpectError,
        log::error!("parse func currencies.pick_specified_num_currency return value error")
    );

    let mut sign_currencys = Vec::<DigitalCurrencyWrapper>::new();
    let mut pay_lock_ids = Vec::<String>::new();
    for each in &picked_currencys {
        match each {
            CurrencyEntity::AvailEntity {
                id,
                owner_uid: _,
                value: _,
                currency,
                currency_str: _,
                txid: _,
                update_time: _,
                last_owner_id: _,
            } => {
                sign_currencys.push(currency.clone());
                pay_lock_ids.push(id.clone());
            }
            _ => {
                log::error!("currencies return currency not avail type");
                return Err(Error::TXExpectError);
            }
        };
    }

    // 收集锁定的货币，在确认删除，或在失败流程解锁
    tx_payload_addr
        .send(MemFnTXSetPayLockCurrencys {
            tx_sm_id: payload.tx_sm_id,
            ids: pay_lock_ids,
        })
        .await??;

    // 对手方
    let oppo_exchanger = &payload.other_exchangers[0];
    let oppo_cert = CertificateSm2::from_bytes(
        &Vec::<u8>::from_hex(&oppo_exchanger.cert).expect("data incrrect"),
    )
    .expect("data incrrect");

    let signed_transactions: SignTransactionResponse = async_parse_check_withlog!(
        call_mod_througth_bus!(
            bus_addr,
            "secret",
            "sign_transaction",
            json!(SignTransactionRequest {
                uid: payload.uid.clone(),
                oppo_cert,
                datas: sign_currencys,
            })
        ),
        Error::TXExpectError,
        log::error!("parse func secret.sign_transaction return value error")
    );

    call_mod_througth_bus!(
        bus_addr,
        "tx_conn",
        "send_tx_msg",
        json!(SendMsgPackage {
            msg: MsgPackage {
                txid: payload.txid,
                data: TransactionSyn {
                    tx_datas: signed_transactions.datas
                }
                .to_msgpack(),
            },
            send_uid: payload.uid,
        })
    );
    Ok(())
}

async fn recv_transaction_syn(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    msg_data: TransactionType,
    payload: TransactionPayload,
) -> Result<(), Error> {
    let recv = TransactionSyn::from_msgpack(msg_data)?;

    tx_payload_addr
        .send(MemFnTXSetRecvWaitConfirmCurrencys {
            tx_sm_id: payload.tx_sm_id,
            currencys: recv.tx_datas,
        })
        .await??;

    Ok(())
}

async fn send_transaction_confirm(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    payload: TransactionPayload,
) -> Result<(), Error> {
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
    msg_data: TransactionType,
    payload: TransactionPayload,
) -> Result<(), Error> {
    for each in payload.recv_wait_confirm_currencys {
        call_mod_througth_bus!(
            bus_addr,
            "currencies",
            "add_currency",
            json!(AddCurrencyParam::WaitConfirmEntity {
                owner_uid: payload.uid.clone(),
                transaction_str: each,
                txid: payload.txid.clone(),
                last_owner_id: payload.oppo_uid.clone(),
            })
        );
    }
    // 若本地两账户互转，插入新货币和删除旧的时序不确定，所以删除仅删除Lock状态的
    for each in payload.pay_lock_currencys {
        call_mod_througth_bus!(
            bus_addr,
            "currencies",
            "remove_pay_lock_currency",
            json!(each)
        );
    }

    tx_payload_addr
        .send(MemFnTXTransactionConfirm {
            tx_sm_id: payload.tx_sm_id,
        })
        .await??;

    Ok(())
}

async fn end_transaction(
    tx_payload_addr: Addr<TXPayloadMgr>,
    bus_addr: Addr<Bus>,
    payload: TransactionPayload,
) -> Result<(), Error> {
    call_mod_througth_bus!(
        bus_addr,
        "transaction",
        "tx_success",
        json!(TXSuccessParam {
            txid: payload.txid,
            uid: payload.uid,
        })
    );

    Ok(())
}
