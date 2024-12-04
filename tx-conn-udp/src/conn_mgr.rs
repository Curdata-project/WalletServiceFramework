use crate::error::Error;
use crate::TXConnModule;
use actix::prelude::*;
use chrono::prelude::Local;
use ewf_core::message::Call;
use futures_channel::mpsc;
use futures_util::future::FutureExt;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cmp::Ordering;
use std::collections::hash_map::HashMap;
use std::collections::BinaryHeap;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::udp::{RecvHalf, SendHalf};
use tokio::net::UdpSocket;
use wallet_common::connect::{MsgPackage, OnConnectNotify, RecvMsgPackage, RouteInfo};
use wallet_common::transaction::TXCloseRequest;

const CHECK_CLOSE_INTERVAL: u64 = 3;
const MAX_CLOSE_TIME_MS: i64 = 3000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrdMsgPackage {
    pub txid: String,
    pub ord_id: u32,
    pub data: Vec<u8>,
}

impl PartialEq for OrdMsgPackage {
    fn eq(&self, other: &Self) -> bool {
        other.ord_id == self.ord_id
    }
}

impl PartialOrd for OrdMsgPackage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.ord_id.partial_cmp(&self.ord_id)
    }
}

impl Eq for OrdMsgPackage {}

impl Ord for OrdMsgPackage {
    fn cmp(&self, other: &Self) -> Ordering {
        other.ord_id.cmp(&self.ord_id)
    }
}

#[derive(Debug, Clone)]
struct SendOrdMsgPackage {
    data: OrdMsgPackage,
    addr: SocketAddr,
}

#[derive(Debug, Clone)]
enum WaitLoopSignal {
    TimeoutCheck,
    // txid
    CloseConn(String),
    Close,
    SendData(SendOrdMsgPackage),
}

struct ListenObj {
    uid: String,
    peer_url: String,

    listen_handle: Option<SpawnHandle>,
    waitloop_sender: Option<mpsc::Sender<WaitLoopSignal>>,
}

#[derive(Debug, Clone)]
struct ConnectObj {
    uid: String,
    oppo_uid: String,

    last_send_id: u32,
}

pub(crate) struct ConnMgr {
    // uid -> listen
    uid_listen: HashMap<String, ListenObj>,

    // txid -> (uid -> conn)
    conn_map: HashMap<String, HashMap<String, ConnectObj>>,

    // 存储路由信息
    // uid -> peer_url
    uid_url: HashMap<String, String>,
    // peer_url -> uid
    url_uid: HashMap<String, String>,

    //向总线发送数据
    conn_addr: Addr<TXConnModule>,
}

impl ConnMgr {
    pub fn new(conn_addr: Addr<TXConnModule>) -> Self {
        Self {
            uid_listen: HashMap::<String, ListenObj>::new(),
            conn_map: HashMap::<String, HashMap<String, ConnectObj>>::new(),
            uid_url: HashMap::<String, String>::new(),
            url_uid: HashMap::<String, String>::new(),
            conn_addr,
        }
    }

    fn bind_listen(&mut self, uid: String, udp_socket: UdpSocket) -> Option<(RecvHalf, SendHalf)> {
        if let Some(_) = self.uid_listen.get(&uid) {
            return None;
        }
        let peer_addr = udp_socket.local_addr().unwrap().to_string();

        log::info!("{} bind_listen at {}", uid, peer_addr);

        let (receiver, sender) = udp_socket.split();

        self.uid_listen.insert(
            uid.clone(),
            ListenObj {
                uid,
                peer_url: peer_addr,
                listen_handle: None,
                waitloop_sender: None,
            },
        );

        Some((receiver, sender))
    }

    fn connect(&mut self, self_uid: String, peer_uid: String, txid: String) -> Result<(), Error> {
        let conn_obj = ConnectObj {
            uid: self_uid.clone(),
            oppo_uid: peer_uid,
            last_send_id: 0,
        };

        if self.conn_map.get(&txid).is_none() {
            self.conn_map
                .insert(txid.clone(), HashMap::<String, ConnectObj>::new());
        }

        let conn_objs = self.conn_map.get_mut(&txid).unwrap();

        conn_objs.insert(self_uid, conn_obj);

        Ok(())
    }

    fn close_conn(&mut self, uid: String, txid: String) {
        // 关闭读端
        if let Some(listen_obj) = self.uid_listen.get_mut(&uid) {
            if let Some(mut sender) = listen_obj.waitloop_sender.clone() {
                sender
                    .try_send(WaitLoopSignal::CloseConn(txid.clone()))
                    .unwrap();
            }
        }
        let is_none: bool = if let Some(conn_objs) = self.conn_map.get_mut(&txid) {
            // 关闭写端
            conn_objs.remove(&uid);

            conn_objs.len() == 0
        } else {
            false
        };

        if is_none {
            self.conn_map.remove(&txid);
        }
    }

    // 服务端口断开，不关注客户端连接，客户端连接通过超时来关
    fn close_bind(&mut self, uid: String) -> Option<SpawnHandle> {
        let ret = if let Some(listen_obj) = self.uid_listen.remove(&uid) {
            if let Some(mut sender) = listen_obj.waitloop_sender {
                sender.try_send(WaitLoopSignal::Close).unwrap();
            }
            listen_obj.listen_handle
        } else {
            None
        };

        for (_, uid_conn) in &mut self.conn_map {
            uid_conn.remove(&uid);
        }

        ret
    }

    fn find_uid_by_addr(&self, addr: &str) -> Option<&String> {
        self.url_uid.get(addr)
    }
}

impl Actor for ConnMgr {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        fn conn_close_check_task(_self: &mut ConnMgr, ctx: &mut Context<ConnMgr>) {
            for listen_obj in _self.uid_listen.values() {
                if let Some(mut sender) = listen_obj.waitloop_sender.clone() {
                    sender.try_send(WaitLoopSignal::TimeoutCheck).unwrap();
                }
            }
        }

        // 启动定时器关闭死链接
        ctx.run_interval(
            Duration::new(CHECK_CLOSE_INTERVAL, 0),
            conn_close_check_task,
        );
    }
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<(), Error>")]
pub(crate) struct MemFnBindListenParam {
    pub uid: String,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<(), Error>")]
pub(crate) struct MemFnConnectParam {
    pub self_uid: String,
    pub peer_uid: String,
    pub txid: String,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<(), Error>")]
pub(crate) struct MemFnBindUidUrlParam {
    pub uid: String,
    pub url: String,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<(), Error>")]
pub(crate) struct MemFnOnConnNotifyParam {
    pub uid: String,
    pub oppo_peer_uid: String,
    pub txid: String,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<String, Error>")]
pub(crate) struct MemFnFindUidByUrlParam {
    pub url: String,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "bool")]
pub(crate) struct MemFnPutRecvMsgPackParam {
    pub self_uid: String,
    pub msg: OrdMsgPackage,
}

// result (uid, msg)
#[derive(Debug, Message, Clone)]
#[rtype(result = "(String, OrdMsgPackage)")]
pub(crate) struct MemFnTryGetOrdMsgPackParam {
    pub self_uid: String,
    pub txid: String,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "()")]
pub(crate) struct MemFnCloseParam {
    pub uid: String,
    pub txid: String,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "()")]
pub(crate) struct MemFnCloseBindParam {
    pub uid: String,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<(), Error>")]
pub(crate) struct MemFnSendParam {
    pub send_uid: String,
    pub txid: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<Vec<RouteInfo>, Error>")]
pub(crate) struct MemFnGetRouteInfosParam {}

impl Handler<MemFnBindListenParam> for ConnMgr {
    type Result = ResponseActFuture<Self, Result<(), Error>>;
    fn handle(&mut self, param: MemFnBindListenParam, ctx: &mut Context<Self>) -> Self::Result {
        let conn_addr = self.conn_addr.clone();
        let self_addr = ctx.address();
        let self_uid = param.uid.clone();

        let get_bind_task = UdpSocket::bind("127.0.0.1:0");

        Box::pin(actix::fut::wrap_future::<_, Self>(get_bind_task).map(move |result, actor, ctx| {
            let udp_socket = match result {
                Ok(udp_socket) => udp_socket,
                Err(_) => return Err(Error::TXBindError),
            };

            let (mut recv_half, mut send_half) = actor.bind_listen(param.uid.clone(), udp_socket).ok_or(Error::TXBindError)?;

            let (signal_sender, mut signal_receiver) = mpsc::channel::<WaitLoopSignal>(10);

            let send_to_other = actix::fut::wrap_future::<_, Self>(async move {
                let mut buf = [0u8; 65535];

                #[derive(Clone)]
                struct WaitOrdMsgObj{
                    heap: BinaryHeap<OrdMsgPackage>,
                    wait_ord_id: u32,
                    last_ord_time: i64,
                }

                // txid -> obj，可考虑采用lru结构替换加速超时淘汰，先暴力迭代
                let mut ord_ids = HashMap::<String, WaitOrdMsgObj>::new();
                loop {
                    futures_util::select! {
                        recv_result = recv_half.recv_from(&mut buf).fuse() => {
                            let (sz, recv_addr) = match recv_result{
                                Ok((sz, recv_addr)) => (sz, recv_addr),
                                Err(_) => continue,
                            };

                            let oppo_peer_uid = match self_addr.send(MemFnFindUidByUrlParam{url: recv_addr.to_string()}).await {
                                Ok(Ok(oppo_peer_uid)) => oppo_peer_uid,
                                _ => continue,
                            };

                            let ord_msg: OrdMsgPackage = match bincode::deserialize(&buf[..sz]) {
                                Ok(ord_msg) => ord_msg,
                                Err(_) => continue,
                            };

                            let wait_ordmsg_obj = if ord_msg.ord_id == 0 {
                                let mut heap = BinaryHeap::<OrdMsgPackage>::new();
                                heap.push(ord_msg.clone());

                                ord_ids.insert(ord_msg.txid.clone(), WaitOrdMsgObj{
                                    heap,
                                    wait_ord_id: 0,
                                    last_ord_time: Local::now().timestamp_millis(),
                                });

                                // TODO txid冲突解决
                                let oppo_peer_uid = match self_addr.send(MemFnOnConnNotifyParam{
                                    uid: self_uid.clone(),
                                    oppo_peer_uid: oppo_peer_uid.to_string(),
                                    txid: ord_msg.txid.clone(),
                                }).await {
                                    Ok(Ok(oppo_peer_uid)) => oppo_peer_uid,
                                    _ => continue,
                                };

                                ord_ids.get_mut(&ord_msg.txid).unwrap()
                            }
                            else{
                                let mut wait_ordmsg_obj = match ord_ids.get_mut(&ord_msg.txid) {
                                    Some(heap) => heap,
                                    // 忽略超时后到达的信息，也可能是比第一个报文先到的，
                                    None => continue,
                                };
                                wait_ordmsg_obj.heap.push(ord_msg.clone());

                                wait_ordmsg_obj
                            };

                            // 排队取出
                            while let Some(min_ord_msg) = wait_ordmsg_obj.heap.peek(){
                                if wait_ordmsg_obj.wait_ord_id != min_ord_msg.ord_id {
                                    break;
                                }
                                let min_ord_msg = wait_ordmsg_obj.heap.pop().unwrap();
                                conn_addr
                                .send(Call {
                                    method: "recv_tx_msg".to_string(),
                                    args: json!(RecvMsgPackage {
                                        msg: MsgPackage {
                                            txid: ord_msg.txid.clone(),
                                            data: min_ord_msg.data,
                                        },
                                        recv_uid: self_uid.clone(),
                                    }),
                                })
                                .await
                                .unwrap()
                                .unwrap();

                                wait_ordmsg_obj.wait_ord_id +=1;
                            }
                            wait_ordmsg_obj.last_ord_time = Local::now().timestamp_millis();
                        },
                        singal = signal_receiver.next().fuse() => match singal{
                            // TODO 读端超时检查，控制连接层超时
                            Some(WaitLoopSignal::TimeoutCheck) => {
                                let update_time = Local::now().timestamp_millis() - MAX_CLOSE_TIME_MS;

                                let mut closes = Vec::<String>::new();
                                for (k, v) in ord_ids.iter() {
                                    if v.last_ord_time < update_time {
                                        self_addr.send(MemFnCloseParam{
                                            uid: self_uid.clone(),
                                            txid: k.to_string(),
                                        }).await.unwrap_or_default();

                                        conn_addr.do_send(Call {
                                            method: "tx_close".to_string(),
                                            args: json!(TXCloseRequest {
                                                txid: k.to_string(),
                                                uid: self_uid.clone(),
                                                reason: "timeout, close by tx-conn-udp".to_string(),
                                            }),
                                        });

                                        closes.push(k.to_string());
                                    }
                                }

                                for each in closes {
                                    ord_ids.remove(&each);
                                }
                            }
                            Some(WaitLoopSignal::SendData(send_data)) => {
                                let data = bincode::serialize(&send_data.data).unwrap();
                                send_half.send_to(&data, &send_data.addr).await.unwrap_or_default();
                            }
                            Some(WaitLoopSignal::CloseConn(txid)) => {
                                ord_ids.remove(&txid);
                            }
                            Some(WaitLoopSignal::Close) => {
                                break;
                            }
                            None => {}
                        },
                    };
                }
            });
            let listen_handle = ctx.spawn(send_to_other);

            if let Some(listen_obj) = actor.uid_listen.get_mut(&param.uid.clone()) {
                listen_obj.listen_handle = Some(listen_handle);
                listen_obj.waitloop_sender = Some(signal_sender);
            }

            Ok(())
        }))
    }
}

impl Handler<MemFnConnectParam> for ConnMgr {
    type Result = Result<(), Error>;
    fn handle(&mut self, param: MemFnConnectParam, _ctx: &mut Context<Self>) -> Self::Result {
        let ret = self.connect(
            param.self_uid.clone(),
            param.peer_uid.clone(),
            param.txid.clone(),
        );

        ret
    }
}

impl Handler<MemFnCloseParam> for ConnMgr {
    type Result = ();
    fn handle(&mut self, param: MemFnCloseParam, _ctx: &mut Context<Self>) -> Self::Result {
        self.close_conn(param.uid, param.txid);
    }
}

impl Handler<MemFnCloseBindParam> for ConnMgr {
    type Result = ();
    fn handle(&mut self, param: MemFnCloseBindParam, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(close_handle) = self.close_bind(param.uid.clone()) {
            log::debug!("uid {} close listen", param.uid);
            // TODO self.close_bind实际上尝试从内部信号关闭任务，此处可考虑去掉或延时
            _ctx.cancel_future(close_handle);
        }
    }
}

impl Handler<MemFnFindUidByUrlParam> for ConnMgr {
    type Result = Result<String, Error>;
    fn handle(&mut self, param: MemFnFindUidByUrlParam, _ctx: &mut Context<Self>) -> Self::Result {
        match self.find_uid_by_addr(&param.url) {
            Some(oppo_peer_uid) => Ok(oppo_peer_uid.to_string()),
            None => Err(Error::TXRouteInfoNotFound),
        }
    }
}

impl Handler<MemFnBindUidUrlParam> for ConnMgr {
    type Result = Result<(), Error>;
    fn handle(&mut self, param: MemFnBindUidUrlParam, _ctx: &mut Context<Self>) -> Self::Result {
        self.uid_url.insert(param.uid.clone(), param.url.clone());
        self.url_uid.insert(param.url, param.uid);

        Ok(())
    }
}

impl Handler<MemFnOnConnNotifyParam> for ConnMgr {
    type Result = Result<(), Error>;
    fn handle(&mut self, param: MemFnOnConnNotifyParam, _ctx: &mut Context<Self>) -> Self::Result {
        let conn_addr = self.conn_addr.clone();

        let is_on_conn = match self.conn_map.get(&param.txid) {
            Some(conn_objs) => match conn_objs.get(&param.uid) {
                Some(_) => false,
                None => true,
            },
            None => true,
        };

        if is_on_conn {
            if self.conn_map.get(&param.txid).is_none() {
                self.conn_map
                    .insert(param.txid.clone(), HashMap::<String, ConnectObj>::new());
            }

            let conn_objs = self.conn_map.get_mut(&param.txid).unwrap();

            let conn_obj = ConnectObj {
                uid: param.uid.clone(),
                oppo_uid: param.oppo_peer_uid.clone(),
                last_send_id: 0,
            };

            conn_objs.insert(param.uid.clone(), conn_obj);

            conn_addr.do_send(Call {
                method: "on_connect".to_string(),
                args: json!(OnConnectNotify {
                    uid: param.uid,
                    oppo_peer_uid: param.oppo_peer_uid,
                    txid: param.txid,
                }),
            });
        }

        Ok(())
    }
}

impl Handler<MemFnSendParam> for ConnMgr {
    type Result = Result<(), Error>;
    fn handle(&mut self, param: MemFnSendParam, _ctx: &mut Context<Self>) -> Self::Result {
        let mut conn_obj = match self.conn_map.get_mut(&param.txid) {
            Some(conn_objs) => match conn_objs.get_mut(&param.send_uid) {
                Some(conn_obj) => conn_obj,
                None => return Err(Error::TXConnectBroken),
            },
            None => return Err(Error::TXConnectBroken),
        };

        let listen_obj = match self.uid_listen.get(&param.send_uid) {
            Some(listen_obj) => listen_obj,
            None => return Err(Error::TXConnectBroken),
        };

        let oppo_addr = match self.uid_url.get(&conn_obj.oppo_uid) {
            Some(oppo_url) => match oppo_url.parse() {
                Ok(oppo_addr) => oppo_addr,
                Err(_) => return Err(Error::TXConnectUrlUnvalid),
            },
            None => return Err(Error::TXConnectBroken),
        };

        let cur_send_id = conn_obj.last_send_id;
        conn_obj.last_send_id += 1;

        if let Some(mut sender) = listen_obj.waitloop_sender.clone() {
            sender
                .try_send(WaitLoopSignal::SendData(SendOrdMsgPackage {
                    data: OrdMsgPackage {
                        txid: param.txid,
                        ord_id: cur_send_id,
                        data: param.data,
                    },
                    addr: oppo_addr,
                }))
                .unwrap();
        }

        Ok(())
    }
}

impl Handler<MemFnGetRouteInfosParam> for ConnMgr {
    type Result = Result<Vec<RouteInfo>, Error>;
    fn handle(&mut self, _: MemFnGetRouteInfosParam, _ctx: &mut Context<Self>) -> Self::Result {
        let mut ret = Vec::<RouteInfo>::new();

        for each in self.uid_listen.values() {
            ret.push(RouteInfo {
                uid: each.uid.clone(),
                url: each.peer_url.clone(),
            });
        }

        Ok(ret)
    }
}
