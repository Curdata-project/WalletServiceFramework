use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use actix::prelude::*;
use crate::WebSocketModule;
use ewf_core::{Call};
use jsonrpc_lite::Error as JsonRpcError;
use serde_json::Value;
use serde::{Serialize, Deserialize};
use jsonrpc_lite::JsonRpc;

type WebSockWriteHalf = SplitSink<WebSocketStream<TcpStream>, Message>;
type WebSockReadHalf = SplitStream<WebSocketStream<TcpStream>>;


const REQ_QUEUE_LEN: usize = 10;

pub struct WSServer{
    listener: TcpListener,
}

impl WSServer {
    pub async fn bind(bind_transport: String) -> Result<Self, String> {
        let listener = TcpListener::bind(&bind_transport)
            .await
            .map_err(|err| err.to_string())?;

        log::info!("Listening on: {}", &bind_transport);

        let instance = Self { listener };

        Ok(instance)
    }

    pub async fn listen_loop(mut self, redirecter: Addr<WebSocketModule>) {
        while let Ok((stream, _)) = self.listener.accept().await {
            let redirecter_ = redirecter.clone();
            actix::spawn(async move {
                if let Err(err) = Self::client_loop(stream, redirecter_).await {
                    log::warn!("{}", err);
                }
            });
        }
    }

    async fn client_loop(stream: TcpStream, redirecter: Addr<WebSocketModule>) -> Result<(), String> {
        let peer = stream
            .peer_addr()
            .map_err(|err| format!("get client peer_addr error, with info: {}", err))?;

        let ws_stream = accept_async(stream)
            .await
            .map_err(|err| format!("ws_stream accept error, with info: {}", err))?;

        log::info!("client {} connect", peer);
        let (write_half, read_half) = ws_stream.split();

        let (req_pipe_in, req_pipe_out) = mpsc::channel(REQ_QUEUE_LEN);
        let (resp_pipe_in, resp_pipe_out) = mpsc::channel(REQ_QUEUE_LEN);

        tokio::select! {
            _ = Self::dispatch_loop(redirecter, req_pipe_out, resp_pipe_in) => {
                log::info!("client {} close because dispatch_loop", peer);
            },
            _ = Self::read_half_loop(read_half, req_pipe_in) => {
                log::info!("client {} close because read_half", peer);
            },
            _ = Self::write_half_loop(write_half, resp_pipe_out) => {
                log::info!("client {} close because write_half", peer);
            },
        };

        Ok(())
    }

    async fn read_half_loop(mut read_half: WebSockReadHalf, mut req_pipe_in: mpsc::Sender<String>) {
        while let Some(ans) = read_half.next().await {
            match ans {
                Err(_) => {
                    return;
                }
                Ok(Message::Text(msg_str)) => {
                    if let Err(_) = req_pipe_in.send(msg_str).await {
                        return;
                    }
                }
                Ok(Message::Ping(_)) => log::debug!("recv message ping/pong"),
                Ok(Message::Pong(_)) => log::debug!("recv message ping/pong"),
                Ok(_) => log::debug!("data format not String, ignore this item"),
            }
        }
    }

    async fn write_half_loop(
        mut write_half: WebSockWriteHalf,
        mut resp_pipe_out: mpsc::Receiver<String>,
    ) {
        while let Some(msg_str) = resp_pipe_out.recv().await {
            if let Err(_) = write_half.send(Message::Text(msg_str)).await {
                return;
            }
        }
    }

    async fn dispatch_loop(
        redirecter: Addr<WebSocketModule>,
        mut req_pipe: mpsc::Receiver<String>,
        mut resp_pipe: mpsc::Sender<String>,
    ) {
        while let Some(req_str) = req_pipe.recv().await {
            if let Err(_) = resp_pipe.send(route_jsonrpc(redirecter.clone(), &req_str).await).await {
                // 处理完客户端已断开，忽略
                return;
            }
        }
    }
}


#[derive(Deserialize, Debug)]
pub struct Request {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
    pub id: i64,
}

/// 传入一个Value格式的json-rpc单独请求
///   立刻返回响应执行Future或者错误结果
pub async fn route_once(
    redirecter: Addr<WebSocketModule>, 
    req: Value,
) -> Value {
    let req: Request = serde_json::from_value(req).unwrap();
    let resp = redirecter.send(Call{
        method: req.method,
        args: req.params,
    }).await.unwrap().unwrap();

    resp
}

/// 传入jsonrpc请求
///   返回结果
pub async fn route_jsonrpc(redirecter: Addr<WebSocketModule>, req_str: &str) -> String {
    let req: Value = match serde_json::from_str(req_str) {
        Ok(req) => req,
        Err(_) => {
            return serde_json::to_value(JsonRpc::error((), JsonRpcError::parse_error()))
                .unwrap()
                .to_string()
        }
    };
    let resp = match req {
        Value::Object(_) => route_once(redirecter, req).await.to_string(),
        Value::Array(array) => {
            "".to_string()
        }
        _ => {
            return serde_json::to_value(JsonRpc::error((), JsonRpcError::parse_error()))
                .unwrap()
                .to_string()
        }
    };

    resp.to_string()
}
