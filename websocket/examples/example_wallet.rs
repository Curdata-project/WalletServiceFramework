extern crate websocket;

use actix::prelude::*;
use currencies::CurrenciesModule;
use ewf_core::states::WalletMachine;
use ewf_core::Bus;
use prepare::PrepareModule;
use secret::SecretModule;
use websocket::WebSocketModule;
use history::HistoryModule;
use user::UserModule;

fn start_sm_wallet() {
    use env_logger::Env;
    env_logger::from_env(Env::default().default_filter_or("warn")).init();

    let mut wallet_bus: Bus = Bus::new();

    let ws_server = WebSocketModule::new("127.0.0.1:9000".to_string());
    let currencies = CurrenciesModule::new("test.db".to_string()).unwrap();
    let secret = SecretModule::new("test.db".to_string()).unwrap();
    let history = HistoryModule::new("test.db".to_string()).unwrap();
    let user = UserModule::new("test.db".to_string()).unwrap();
    let prepare = PrepareModule::new(vec!["currencies", "webscoket_jsonrpc", "secret", "history", "user"]);

    wallet_bus
        .machine(WalletMachine::default())
        .module(1, ws_server)
        .module(2, currencies)
        .module(3, secret)
        .module(4, history)
        .module(5, user)
        .module(6, prepare);

    wallet_bus.start();
}

fn main() {
    let io_result = actix::System::run(|| {
        start_sm_wallet();
    });

    log::info!("System exit with {:?}", io_result);
}
