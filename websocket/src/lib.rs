mod error;
mod currencies;

use serde_json::Value;
use jsonrpc_core::route::Route;
use jsonrpc_lite::Error as JsonRpcError;
use jsonrpc_websocket::WsServer;
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;
use std::sync::{Arc, RwLock};

use actix::prelude::*;
use ewf_core::error::Error as EwfError;
use ewf_core::{Call, Event, Module, Transition, CallQuery};

use crate::currencies::{self as currencies_s, CurrencyResource};


pub struct WebSocketModule{
    bind_transport: String,
}

impl WebSocketModule {
    pub fn new(bind_transport: String) -> Self {
        Self{
            bind_transport
        }
    }
}



impl Actor for WebSocketModule {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {

    }
}

impl Handler<Call> for WebSocketModule {
    type Result = ResponseFuture<Result<Value, EwfError>>;
    fn handle(&mut self, _msg: Call, _ctx: &mut Context<Self>) -> Self::Result {
        Box::pin(async move {
            
            Ok(Value::default())
        })
    }
}


impl Handler<Event> for WebSocketModule {
    type Result = ResponseFuture<Result<(), EwfError>>;
    fn handle(&mut self, _msg: Event, _ctx: &mut Context<Self>) -> Self::Result {
        let bind_transport = self.bind_transport.clone();

        Box::pin(async move {
            let event: &str = &_msg.event;
            match event {
                "Start" => {
                    let currencies: Recipient<Call> = _msg.addr.send(CallQuery{module: "currencies".to_string()}).await??;

                    tokio::spawn(async move {
                        let route: Arc<Route> = Arc::new(
                            Route::new()
                                .data(CurrencyResource::new(currencies))
                                .to("currency.ids.detail".to_string(), currencies_s::get_detail_by_ids),
                        );

                        match WsServer::bind(bind_transport).await {
                            Ok(ws_server) => ws_server.listen_loop(route).await,
                            Err(err) => log::error!("{:?}", err),
                        }
                    });
                }
                // no care this event, ignore
                _ => return Ok(()),
            }

            Ok(())
        })
    }
}

impl fmt::Debug for WebSocketModule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("{{ {} {} }}", self.name(), self.version()))
    }
}

impl Module for WebSocketModule {
    fn name(&self) -> String {
        "webscoket_jsonrpc".to_string()
    }

    fn version(&self) -> String {
        "0.1".to_string()
    }
}
