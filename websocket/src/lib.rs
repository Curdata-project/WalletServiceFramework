#![feature(async_closure)]

mod error;
mod currencies_resource;

use serde_json::{Value, json};
use jsonrpc_core::route::Route;
use jsonrpc_lite::Error as JsonRpcError;
use jsonrpc_websocket::WsServer;
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;
use std::sync::{Arc, RwLock, Mutex};

use actix::prelude::*;
use ewf_core::error::Error as EwfError;
use ewf_core::{Call, Event, Module, Transition, CallQuery};

use std::pin::Pin;

use crate::currencies_resource::{CurrencyResource};


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

use crossbeam::channel;
use once_cell::sync::Lazy;

/// A queue that holds scheduled tasks.
pub static QUEUE: Lazy<(channel::Sender<Task>, channel::Receiver<Task>)> = Lazy::new(|| {
    // Create a queue.
    let (sender, receiver) = channel::unbounded::<Task>();

    (sender, receiver)
});

type Task = async_task::Task<()>;

type JoinHandle<R> = Pin<Box<dyn Future<Output = R> + Send>>;

fn spawn_call_task<F, R>(future: F) -> JoinHandle<R>
where
    F: Future<Output = R> + 'static,
    R: Send + 'static,
{
    let (task, handle) = async_task::spawn_local(future, |t| QUEUE.0.send(t).unwrap(), ());
    task.schedule();

    Box::pin(async { handle.await.unwrap() })
}

struct SyncActorRunCallTask;

impl Actor for SyncActorRunCallTask {
    type Context = SyncContext<Self>;

    fn started(&mut self, _ctx: &mut SyncContext<Self>) {
        QUEUE.1.iter().for_each(|task| { task.run(); })
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
            let id = _msg.id;
            let addr = _msg.addr.clone();
            match event {
                "Start" => {
                    actix::spawn(async move {
                        SyncArbiter::start(2, || SyncActorRunCallTask);
                    });

                    actix::spawn(async move {
                        let route: Arc<Route> = Arc::new(
                            Route::new()
                                .data(CurrencyResource::new(addr))
                                .to("currency.ids.detail".to_string(), currencies_resource::get_detail_by_ids),
                        );

                        match WsServer::bind(bind_transport).await {
                            Ok(ws_server) => ws_server.listen_loop(route).await,
                            Err(err) => log::error!("{:?}", err),
                        }
                    });

                    _msg.addr.send(Transition {
                        id,
                        transition: "InitalSuccess".to_string(),
                    })
                    .await??;
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
        f.write_fmt(format_args!("{{ {} {} {} }}", self.name(), self.version(), self.bind_transport))
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


#[cfg(test)]
mod tests {
    extern crate currencies;
    
    use super::*;
    use ewf_core::states::WalletMachine;
    use ewf_core::{Bus, Transition};
    use currencies::CurrenciesModule;
    use std::time::{Duration, Instant};
    use tokio::time::delay_for;

    #[actix_rt::test]
    async fn test_websocket() {
        let mut wallet_bus: Bus = Bus::new();

        let currencies = CurrenciesModule::new("db_data".to_string()).unwrap();
        let ws_server = WebSocketModule::new("127.0.0.1:9000".to_string());

        wallet_bus
            .machine(WalletMachine::default())
            .module(1, currencies)
            .module(2, ws_server);

        let addr = wallet_bus.start();
        addr.send(Transition {
            id: 0,
            transition: "Starting".to_string(),
        })
        .await
        .unwrap()
        .unwrap();

        delay_for(Duration::from_millis(3600 * 1000)).await;
    }
}
