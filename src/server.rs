use crate::currency_store::{get_detail_by_ids, CurrencyStore};
use crate::network::websock_server::WsServer;
use crate::wallet::Wallet;
use jsonrpc_lite::Error as JsonRpcError;
use jsonrpc_lite::JsonRpc;
use jsonrpc_ws::server::Server;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

fn server_route_error() -> JsonRpcError {
    JsonRpcError {
        code: -32500,
        message: "Server Internal Route error".to_string(),
        data: None,
    }
}

/// 传入jsonrpc请求
///   返回结果
pub async fn route_jsonrpc(server: Arc<Server>, req_str: &str) -> String {
    let req: Value = match serde_json::from_str(req_str) {
        Ok(req) => req,
        Err(_) => {
            return serde_json::to_value(JsonRpc::error((), JsonRpcError::parse_error()))
                .unwrap()
                .to_string()
        }
    };
    let resp = match req {
        Value::Object(_) => match server.route_once(req).await {
            Ok(fut) => fut.await,
            Err(err) => err,
        },
        Value::Array(array) => {
            let localtask = tokio::task::LocalSet::new();
            let share_outputs = Arc::new(Mutex::new(Vec::<Value>::new()));

            for each in array {
                let inner_server = Arc::downgrade(&server);
                let share_outputs = share_outputs.clone();

                localtask.spawn_local(async move {
                    // task开始执行是尝试获取server对象
                    let output = match inner_server.upgrade() {
                        Some(server) => match server.route_once(each).await {
                            Ok(fut) => fut.await,
                            Err(err) => err,
                        },
                        None => serde_json::to_value(server_route_error()).unwrap(),
                    };

                    let mut outputs = share_outputs.lock().unwrap();
                    outputs.push(output);
                });
            }
            localtask.await;

            // TODO 内部panic可能要处理
            // outputs Arc持有者只剩下一个，此处取出不会失败，也不考虑失败处理
            let output = if let Ok(outputs) = Arc::try_unwrap(share_outputs) {
                // 锁持有者同理
                outputs.into_inner().unwrap()
            } else {
                panic!("Arc<Mutex<>> into_inner failed");
            };
            Value::Array(output)
        }
        _ => {
            return serde_json::to_value(JsonRpc::error((), JsonRpcError::parse_error()))
                .unwrap()
                .to_string()
        }
    };

    resp.to_string()
}

async fn start_ws_server(bind_transport: String, storage_path: String) {
    let server = Arc::new(
        Server::new()
            .data(Wallet::new())
            .to("currency.ids.detail".to_string(), get_detail_by_ids),
    );

    let dispatch_msg = move |req_pipe: mpsc::Receiver<String>,
                             resp_pipe: mpsc::Sender<String>|
          -> Pin<Box<dyn Future<Output = ()> + Send>> {
        async fn inner(req_pipe: mpsc::Receiver<String>, resp_pipe: mpsc::Sender<String>) {
            while let Some(req_str) = req_pipe.recv().await {
                let resp_str = route_jsonrpc(server, &req_str).await;
                if let Err(_) = resp_pipe.send(resp_str).await {
                    return;
                }
            }
        }
        Box::pin(inner(req_pipe, resp_pipe))
    };

    let mut ws_server = match WsServer::bind(bind_transport).await {
        Ok(ws_server) => ws_server,
        Err(err) => panic!("{}", err),
    };

    ws_server.listen_loop(Box::new(dispatch_msg)).await;
}
