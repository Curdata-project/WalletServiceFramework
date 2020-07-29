use serde_json::Value;
use std::fmt;

use actix::prelude::*;
use ewf_core::error::Error as EwfError;
use ewf_core::{Bus, Call, CallQuery, Event, Module, StartNotify, Transition};
use wallet_common::WALLET_SM_CODE;
use wallet_common::prepare::PrepareParam;


pub struct PrepareModule {
    bus_addr: Option<Addr<Bus>>,
    prepare_cnt: u64,
    /// success num
    prepare_num_s: u64,
    /// failed num
    prepare_num_f: u64,
}

impl PrepareModule {
    pub fn new(prepare_cnt: u64) -> Self {
        Self{
            bus_addr: None,
            prepare_cnt,
            prepare_num_s: 0,
            prepare_num_f: 0,
        }
    }
}

impl Actor for PrepareModule {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {}
}

impl Handler<Call> for PrepareModule {
    type Result = ResponseFuture<Result<Value, EwfError>>;
    fn handle(&mut self, msg: Call, _ctx: &mut Context<Self>) -> Self::Result {
        let bus_addr = self.bus_addr.clone().unwrap();
        let args = msg.args.clone();

        let resolve_inital = move || -> Result<ResponseFuture<Result<Value, EwfError>>, EwfError> {
            let param: PrepareParam = serde_json::from_value(args).map_err(|_| EwfError::CallParamValidFaild)?;
            match param.is_prepare {
                true => self.prepare_num_s += 1,
                false => self.prepare_num_f += 1,
            }

            if self.prepare_num_s + self.prepare_num_f == self.prepare_cnt {
                let inital_ans = match self.prepare_num_s == self.prepare_cnt {
                    true => "InitalSuccess".to_string(),
                    false => "InitalFailed".to_string(),
                };
                let inital_task = Box::pin(async move {
                    bus_addr.send(Transition{ id: WALLET_SM_CODE, transition: inital_ans }).await??;
                    Ok(Value::Null)
                });

                return Ok(inital_task);
            }
            Ok(Box::pin(async move{ Ok(Value::Null) }))
        };

        let method: &str = &msg.method;
        match method {
            "inital" => {
                match resolve_inital() {
                    Ok(result) => result,
                    Err(err) => Box::pin(async move{ Err(err) }),
                }
            }
            _ => return Box::pin(async move{ Err(EwfError::MethodNotFoundError) }),
        }
    }
}

impl Handler<Event> for PrepareModule {
    type Result = ResponseFuture<Result<(), EwfError>>;
    fn handle(&mut self, msg: Event, _ctx: &mut Context<Self>) -> Self::Result {

        let event: &str = &msg.event;
        let id = msg.id;
        match event {
            "Start" => {
                self.prepare_num_s = 0;
                self.prepare_num_f = 0;
            }
            // no care this event, ignore
            _ => {},
        }

        Box::pin(async{ Ok(()) })
    }
}

impl Handler<StartNotify> for PrepareModule {
    type Result = ();
    fn handle(&mut self, msg: StartNotify, _ctx: &mut Context<Self>) -> Self::Result {
        self.bus_addr = Some(msg.addr);
    }
}

impl fmt::Debug for PrepareModule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!(
            "{{ {} {} }}",
            self.name(),
            self.version()
        ))
    }
}

impl Module for PrepareModule {
    fn name(&self) -> String {
        "prepare".to_string()
    }

    fn version(&self) -> String {
        "0.1".to_string()
    }
}

#[cfg(test)]
mod tests {
    extern crate currencies;

    use super::*;
    use currencies::CurrenciesModule;
    use ewf_core::states::WalletMachine;
    use ewf_core::{Bus, Transition};
    use std::time::Duration;
    use tokio::time::delay_for;
    use websocket::WebSocketModule;

    #[actix_rt::test]
    async fn test_prepare() {
        use env_logger::Env;
        env_logger::from_env(Env::default().default_filter_or("warn"))
            .is_test(true)
            .init();

        let mut wallet_bus: Bus = Bus::new();

        let currencies = CurrenciesModule::new("test.db".to_string()).unwrap();
        let ws_server = WebSocketModule::new("127.0.0.1:9000".to_string());
        let prepare = PrepareModule::new(2);

        wallet_bus
            .machine(WalletMachine::default())
            .module(1, currencies)
            .module(2, ws_server)
            .module(3, prepare);

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
