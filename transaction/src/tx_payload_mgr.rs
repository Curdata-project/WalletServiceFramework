use crate::error::Error;
use crate::transaction_msg::CurrencyStat;
use crate::TransactionModule;
use actix::prelude::*;
use chrono::prelude::Local;
use common_structure::digital_currency::DigitalCurrencyWrapper;
use common_structure::get_rng_core;
use ewf_core::message::Call;
use hex::ToHex;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::collections::hash_map::HashMap;
use std::time::Duration;
use wallet_common::connect::{MsgPackage, OnConnectNotify, RecvMsgPackage};
use wallet_common::transaction::{
    CurrencyPlanItem, TXCloseRequest, TXSendRequest, TXSendResponse, TransactionExchangerItem,
};

const CHECK_CLOSE_INTERVAL: u64 = 3;
const MAX_CLOSE_TIME_MS: i64 = 2000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerCurrencyPlan {
    pub uid: String,
    pub item: CurrencyPlanItem,
}

#[derive(Clone)]
pub struct TransactionPayload {
    pub uid: String,
    pub exchangers: Vec<TransactionExchangerItem>,
    pub is_payer: bool,
    pub amount: u64,
    pub oppo_uid: String,

    /// 支付者的零钱，参与支付方案运算，当is_payer为false时有值
    pub pay_currency_stat: Option<CurrencyStat>,

    /// 收款方计算出的或支付者接收的支付计划
    pub currency_plan: Vec<PeerCurrencyPlan>,

    // 使用txid与conn管理交互
    pub txid: String,

    pub tx_sm_id: u64,
    pub last_update_time: i64,
}

impl TransactionPayload {
    fn new(uid: String, tx_sm_id: u64, txid: String) -> Self {
        Self {
            uid,
            exchangers: Vec::<TransactionExchangerItem>::new(),
            is_payer: false,
            amount: 0,
            oppo_uid: "".to_string(),
            pay_currency_stat: None,
            currency_plan: Vec::<PeerCurrencyPlan>::new(),
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

pub(crate) struct TXPayloadMgr {
    tx_sm_datas: HashMap<u64, TransactionPayload>,
    // (txid, uid) -> tx_sm_id
    tx_link: HashMap<(String, String), u64>,

    transaction_addr: Addr<TransactionModule>,
}

impl TXPayloadMgr {
    pub fn new(transaction_addr: Addr<TransactionModule>) -> Self {
        Self {
            tx_sm_datas: HashMap::<u64, TransactionPayload>::new(),
            tx_link: HashMap::<(String, String), u64>::new(),
            transaction_addr,
        }
    }
}

impl Actor for TXPayloadMgr {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        fn close_check_task(_self: &mut TXPayloadMgr, ctx: &mut Context<TXPayloadMgr>) {
            for ((_, uid), tx_sm_id) in _self.tx_link.clone().into_iter() {
                let pay_load = match _self.tx_sm_datas.get(&tx_sm_id) {
                    Some(pay_load) => pay_load,
                    None => continue,
                };

                let now = Local::now().timestamp_millis();

                // 关闭死链接
                if now - pay_load.last_update_time > MAX_CLOSE_TIME_MS {
                    _self.transaction_addr.do_send(Call {
                        method: "tx_close".to_string(),
                        args: json!(TXCloseRequest {
                            txid: pay_load.txid.clone(),
                            uid,
                            reason: "timeout, close by close_check_task".to_string(),
                        }),
                    });
                }
            }
        }

        // 启动定时器关闭死链接
        _ctx.run_interval(Duration::new(CHECK_CLOSE_INTERVAL, 0), close_check_task);
    }
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<MemFnTXPayloadMgrCreateResult, Error>")]
pub(crate) struct MemFnTXPayloadMgrCreate {
    pub uid: String,
    pub tx_sm_id: u64,
    // 有限制(is_tx_sender&&txid.is_none()) ||(!is_tx_sender&&txid)
    pub is_tx_sender: bool,
    pub txid: Option<String>,
}

pub(crate) struct MemFnTXPayloadMgrCreateResult {
    pub txid: String,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<(), Error>")]
pub(crate) struct MemFnTXSetPaymentPlan {
    pub txid: String,
    pub uid: String,
    pub oppo_uid: String,

    pub exchangers: Vec<TransactionExchangerItem>,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "()")]
pub(crate) struct MemFnTXPayloadMgrClose {
    pub txid: String,
    pub uid: String,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<TransactionPayload, Error>")]
pub(crate) struct MemFnTXPayloadGet {
    pub txid: String,
    pub uid: String,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<TransactionPayload, Error>")]
pub(crate) struct MemFnTXPayloadGetBySmid {
    pub tx_sm_id: u64,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<(), Error>")]
pub(crate) struct MemFnTXSetPayCurrencyStat {
    pub tx_sm_id: u64,
    pub currency_stat: CurrencyStat,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<(), Error>")]
pub(crate) struct MemFnTXSetCurrencyPlan {
    pub tx_sm_id: u64,
    pub peer_plan: Vec<PeerCurrencyPlan>,
}

impl Handler<MemFnTXPayloadMgrCreate> for TXPayloadMgr {
    type Result = Result<MemFnTXPayloadMgrCreateResult, Error>;
    fn handle(
        &mut self,
        params: MemFnTXPayloadMgrCreate,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        let cur_tx_id = match params.is_tx_sender {
            true => TransactionPayload::gen_txid(),
            false => params.txid.unwrap(),
        };

        self.tx_sm_datas.insert(
            params.tx_sm_id,
            TransactionPayload::new(params.uid.clone(), params.tx_sm_id, cur_tx_id.clone()),
        );
        self.tx_link
            .insert((cur_tx_id.clone(), params.uid), params.tx_sm_id);

        Ok(MemFnTXPayloadMgrCreateResult { txid: cur_tx_id })
    }
}

impl Handler<MemFnTXSetPaymentPlan> for TXPayloadMgr {
    type Result = Result<(), Error>;
    fn handle(&mut self, params: MemFnTXSetPaymentPlan, _ctx: &mut Context<Self>) -> Self::Result {
        let tx_sm_id = match self.tx_link.get(&(params.txid.clone(), params.uid.clone())) {
            Some(tx_sm_id) => tx_sm_id,
            None => return Err(Error::TXMachineDestoryed),
        }
        .clone();

        let mut iter = params
            .exchangers
            .iter()
            .filter(|each| each.uid == params.uid);
        let user_exchanger = if let Some(exchanger) = iter.next() {
            exchanger
        } else {
            return Err(Error::TXPaymentPlanNotForUser);
        };

        let (is_payer, amount) = if user_exchanger.output > user_exchanger.intput {
            (true, user_exchanger.output - user_exchanger.intput)
        } else {
            (false, user_exchanger.intput - user_exchanger.output)
        };

        if let Some(payload) = self.tx_sm_datas.get_mut(&tx_sm_id) {
            payload.exchangers.extend_from_slice(&params.exchangers[..]);
            payload.is_payer = is_payer;
            payload.amount = amount;
            payload.oppo_uid = params.oppo_uid;
        }

        Ok(())
    }
}

impl Handler<MemFnTXPayloadMgrClose> for TXPayloadMgr {
    type Result = ();
    fn handle(&mut self, params: MemFnTXPayloadMgrClose, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(tx_sm_id) = self
            .tx_link
            .get_mut(&(params.txid.clone(), params.uid.clone()))
        {
            self.tx_sm_datas.remove(&tx_sm_id);
        }
        self.tx_link
            .remove(&(params.txid.clone(), params.uid.clone()));
    }
}

impl Handler<MemFnTXPayloadGet> for TXPayloadMgr {
    type Result = Result<TransactionPayload, Error>;
    fn handle(&mut self, params: MemFnTXPayloadGet, _ctx: &mut Context<Self>) -> Self::Result {
        let tx_sm_id = self
            .tx_link
            .get(&(params.txid.clone(), params.uid.clone()))
            .ok_or(Error::TXMachineDestoryed)?;
        match self.tx_sm_datas.get(&tx_sm_id) {
            Some(ans) => Ok(ans.clone()),
            None => Err(Error::TXMachineDestoryed),
        }
    }
}

impl Handler<MemFnTXPayloadGetBySmid> for TXPayloadMgr {
    type Result = Result<TransactionPayload, Error>;
    fn handle(
        &mut self,
        params: MemFnTXPayloadGetBySmid,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        match self.tx_sm_datas.get(&params.tx_sm_id) {
            Some(ans) => Ok(ans.clone()),
            None => Err(Error::TXMachineDestoryed),
        }
    }
}

impl Handler<MemFnTXSetPayCurrencyStat> for TXPayloadMgr {
    type Result = Result<(), Error>;
    fn handle(
        &mut self,
        params: MemFnTXSetPayCurrencyStat,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        match self.tx_sm_datas.get_mut(&params.tx_sm_id) {
            Some(mut payload) => {
                payload.pay_currency_stat = Some(params.currency_stat);
                Ok(())
            }
            None => Err(Error::TXMachineDestoryed),
        }
    }
}

impl Handler<MemFnTXSetCurrencyPlan> for TXPayloadMgr {
    type Result = Result<(), Error>;
    fn handle(&mut self, params: MemFnTXSetCurrencyPlan, _ctx: &mut Context<Self>) -> Self::Result {
        match self.tx_sm_datas.get_mut(&params.tx_sm_id) {
            Some(mut payload) => {
                payload.currency_plan.extend_from_slice(&params.peer_plan);
                Ok(())
            }
            None => Err(Error::TXMachineDestoryed),
        }
    }
}
