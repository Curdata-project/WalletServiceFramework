extern crate websocket;

use actix::prelude::*;
use currencies::CurrenciesModule;
use ewf_core::states::WalletMachine;
use ewf_core::Bus;
use prepare::PrepareModule;
use secret::SecretModule;
use transaction::TransactionModule;
use tx_conn_local::TXConnModule;
use websocket::WebSocketModule;

fn start_sm_wallet() {
    use env_logger::Env;
    env_logger::from_env(Env::default().default_filter_or("warn")).init();

    let mut wallet_bus: Bus = Bus::new();

    let currencies = CurrenciesModule::new("test.db".to_string()).unwrap();
    let secret = SecretModule::new("test.db".to_string()).unwrap();
    let ws_server = WebSocketModule::new("127.0.0.1:9000".to_string());
    let transaction = TransactionModule::new();
    let tx_conn = TXConnModule::new();
    let prepare = PrepareModule::new(vec![
        "currencies",
        "webscoket_jsonrpc",
        "secret",
        "transaction",
        "tx_conn",
    ]);

    wallet_bus
        .machine(WalletMachine::default())
        .module(1, currencies)
        .module(2, secret)
        .module(3, ws_server)
        .module(4, transaction)
        .module(5, tx_conn)
        .module(6, prepare);

    wallet_bus.start();
}

fn main() {
    let io_result = actix::System::run(|| {
        start_sm_wallet();
    });

    log::info!("System exit with {:?}", io_result);
}
