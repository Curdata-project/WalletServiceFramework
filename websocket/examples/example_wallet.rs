extern crate websocket;

use actix::prelude::*;
use currencies::CurrenciesModule;
use ewf_core::states::WalletMachine;
use ewf_core::Bus;
use history::HistoryModule;
use prepare::PrepareModule;
use secret::SecretModule;
use transaction::TransactionModule;
use tx_conn_udp::TXConnModule;
use user::UserModule;
use websocket::WebSocketModule;

fn start_sm_wallet() {
    use env_logger::Env;
    env_logger::from_env(Env::default().default_filter_or("warn")).init();

    let mut wallet_bus: Bus = Bus::new();

    let ws_server = WebSocketModule::new("127.0.0.1:9000".to_string());
    let currencies = CurrenciesModule::new("test.db".to_string()).unwrap();
    let secret = SecretModule::new("test.db".to_string()).unwrap();
    let transaction = TransactionModule::new();
    let history = HistoryModule::new("test.db".to_string()).unwrap();
    let user = UserModule::new("test.db".to_string()).unwrap();
    let tx_conn = TXConnModule::new();
    let prepare = PrepareModule::new();

    // 启动顺序依赖
    //  secret依赖user，注册后用户信息填写
    //        弱依赖tx_conn，注册成功立刻创建交易通道
    //  tx_conn依赖secret，对本地密钥用户进行交易通道创建
    //  transaction依赖tx_conn，交易依赖交易通道
    //                 history，交易历史记录
    //                 user，用户信息交换
    //                 secret，交易签名
    //                 currencies，货币存储
    wallet_bus
        .machine(WalletMachine::default())
        .module(1, currencies)
        .module(2, history)
        .module(3, user)
        .module(4, secret)
        .module(5, tx_conn)
        .module(6, transaction)
        .module(7, ws_server)
        .module(8, prepare);

    wallet_bus.start();
}

fn main() {
    let io_result = actix::System::run(|| {
        start_sm_wallet();
    });

    log::info!("System exit with {:?}", io_result);
}
