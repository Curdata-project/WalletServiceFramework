use crate::error::Error;
use crate::TXConnModule;
use actix::prelude::*;
use ewf_core::message::Call;
use futures::channel::mpsc;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use serde_json::Value;
use std::collections::hash_map::HashMap;
use wallet_common::connect::*;
use chrono::prelude::Local;

#[derive(Debug)]
struct ListenObj {
    peer_code: u64,
    uid: String,

    listen: Option<mpsc::Receiver<MsgPackage>>,
    // spawn了mpsc::Receiver<MsgPackage>产生的handle
    listen_handle: Option<SpawnHandle>,
}

#[derive(Debug, Clone)]
struct ConnectObj {
    uid: String,
    oppo_uid: String,

    // 连接建立时间 TODO可考虑根据此字段定时断开废弃连接
    build_time: i64,

    connect_sender: mpsc::Sender<MsgPackage>,
    connected_sender: mpsc::Sender<MsgPackage>,
}

pub(crate) struct ConnMgr {
    // 以下为连接<->监听关联, 主要用作组成ConnectObj信息
    // peer_code-> accept conn， 此accept conn为原型，复制使用
    conn_bridge: HashMap<u64, mpsc::Sender<MsgPackage>>,

    // 以下为监听管理peer_addr, uid到具体监听映射
    // uid -> peer_addr
    uid_listen: HashMap<String, ListenObj>,
    // peer_addr -> uid
    addr_listen: HashMap<u64, String>,

    peer_code_static: u64,

    // 以下为连接管理, 管理txid与具体连接映射
    // txid -> ConnectObj
    tx_conn: HashMap<String, ConnectObj>,

    //向总线发送数据
    conn_addr: Addr<TXConnModule>,
}

impl ConnMgr {
    pub fn new(conn_addr: Addr<TXConnModule>) -> Self {
        Self {
            conn_bridge: HashMap::<u64, mpsc::Sender<MsgPackage>>::new(),
            uid_listen: HashMap::<String, ListenObj>::new(),
            addr_listen: HashMap::<u64, String>::new(),
            peer_code_static: 0,
            tx_conn: HashMap::<String, ConnectObj>::new(),
            conn_addr,
        }
    }

    fn bind_listen(&mut self, uid: String) {
        if let Some(_) = self.uid_listen.get(&uid) {
            return;
        }
        let (sender, receiver) = mpsc::channel::<MsgPackage>(10);

        let new_peer_code = self.peer_code_static;
        self.peer_code_static += 1;

        let listen_obj = ListenObj {
            peer_code: new_peer_code,
            uid: uid.clone(),
            listen: Some(receiver),
            listen_handle: None,
        };

        self.uid_listen.insert(uid.clone(), listen_obj);
        self.addr_listen.insert(new_peer_code, uid);
        self.conn_bridge.insert(new_peer_code, sender);
    }

    fn connect(&mut self, self_uid: String, peer_uid: String, txid: String) -> Result<(), Error> {
        if !self.tx_conn.get(&txid).is_none() {
            return Err(Error::TXConnectCollision);
        }

        let listen_obj = self
            .uid_listen
            .get(&peer_uid)
            .ok_or(Error::TXConnectError)?;
        let connect_sender = self
            .conn_bridge
            .get(&listen_obj.peer_code)
            .ok_or(Error::TXConnectError)?;

        let listen_obj = self
            .uid_listen
            .get(&self_uid)
            .ok_or(Error::TXConnectError)?;
        let connected_sender = self
            .conn_bridge
            .get(&listen_obj.peer_code)
            .ok_or(Error::TXConnectError)?;

        let connect_obj = ConnectObj {
            uid: self_uid,
            oppo_uid: peer_uid,
            build_time: Local::now().timestamp_millis(),
            connect_sender: connect_sender.clone(),
            connected_sender: connected_sender.clone(),
        };

        self.tx_conn.insert(txid, connect_obj);

        Ok(())
    }

    fn close_conn(&mut self, txid: String) {
        self.tx_conn.remove(&txid);
    }

    // 服务端口断开，不关注客户端连接，客户端连接通过超时来关
    fn close_bind(&mut self, uid: String) -> Option<SpawnHandle> {
        let (peer_code, listen_handle) = match self.uid_listen.remove(&uid) {
            Some(mut listen_obj) => {
                if let Some(listen_handle) = listen_obj.listen_handle.take() {
                    (listen_obj.peer_code, Some(listen_handle))
                } else {
                    (listen_obj.peer_code, None)
                }
            }
            None => return None,
        };
        self.addr_listen.remove(&peer_code);
        self.conn_bridge.remove(&peer_code);

        listen_handle
    }
}

impl Actor for ConnMgr {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {}
}

#[derive(Debug, Message, Clone)]
#[rtype(result = "()")]
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
#[rtype(result = "()")]
pub(crate) struct MemFnCloseParam {
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
    pub data: Value,
}

impl Handler<MemFnBindListenParam> for ConnMgr {
    type Result = ();
    fn handle(&mut self, param: MemFnBindListenParam, _ctx: &mut Context<Self>) -> Self::Result {
        let conn_addr = self.conn_addr.clone();

        self.bind_listen(param.uid.clone());

        if let Some(listen_obj) = self.uid_listen.get_mut(&param.uid) {
            let recv_uid = listen_obj.uid.clone();
            let mut receiver = match listen_obj.listen.take() {
                Some(receiver) => receiver,
                None => return,
            };
            log::debug!("start listen...");

            let send_to_other = actix::fut::wrap_future::<_, Self>(async move {
                while let Some(msg) = receiver.next().await {
                    conn_addr
                        .send(Call {
                            method: "recv_tx_msg".to_string(),
                            args: json!(RecvMsgPackage {
                                msg: MsgPackage {
                                    txid: msg.txid,
                                    data: msg.data,
                                },
                                recv_uid: recv_uid.clone(),
                            }),
                        })
                        .await
                        .unwrap()
                        .unwrap();
                }
            });

            listen_obj.listen_handle = Some(_ctx.spawn(send_to_other));
        }
    }
}

impl Handler<MemFnConnectParam> for ConnMgr {
    type Result = Result<(), Error>;
    fn handle(&mut self, param: MemFnConnectParam, _ctx: &mut Context<Self>) -> Self::Result {
        self.connect(param.self_uid, param.peer_uid, param.txid)
    }
}

impl Handler<MemFnCloseParam> for ConnMgr {
    type Result = ();
    fn handle(&mut self, param: MemFnCloseParam, _ctx: &mut Context<Self>) -> Self::Result {
        self.close_conn(param.txid);
    }
}

impl Handler<MemFnCloseBindParam> for ConnMgr {
    type Result = ();
    fn handle(&mut self, param: MemFnCloseBindParam, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(close_handle) = self.close_bind(param.uid) {
            log::debug!("close listen...");
            _ctx.cancel_future(close_handle);
        }
    }
}

impl Handler<MemFnSendParam> for ConnMgr {
    type Result = ResponseFuture<Result<(), Error>>;
    fn handle(&mut self, param: MemFnSendParam, _ctx: &mut Context<Self>) -> Self::Result {
        let conn_obj = match self.tx_conn.get(&param.txid) {
            Some(conn_obj) => conn_obj,
            None => return Box::pin(async move { Err(Error::TXConnectBroken) }),
        }
        .clone();
        Box::pin(async move {
            let send_uid = &param.send_uid;
            let mut use_sender = if send_uid == &conn_obj.uid {
                conn_obj.connect_sender
            } else {
                conn_obj.connected_sender
            };

            use_sender
                .send(MsgPackage {
                    txid: param.txid,
                    data: param.data,
                })
                .await
                .map_err(|_| Error::TXConnectBroken)
        })
    }
}
