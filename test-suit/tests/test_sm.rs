use currencies::CurrenciesModule;
use keypair::KeypairModule;
use wallet_service_framework::states::WalletMachine;
use wallet_service_framework::{Bus, Error as FrameworkError, Machine};

#[test]
fn test_wallet_sm() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Info)
        .try_init();

    let path = "db_data";

    // 构建货币管理模块
    let currencies = match CurrenciesModule::new(path.to_string()) {
        Ok(currencies) => currencies,
        Err(err) => panic!("module instance error"),
    };

    // 构建密钥管理模块
    let key_pair = match KeypairModule::new(path.to_string()) {
        Ok(key_pair) => key_pair,
        Err(err) => panic!("module instance error"),
    };

    // 启动WalletMachine
    let mut bus = Bus::new()
        .registe_machine(Box::new(WalletMachine::default()))
        .registe_module(1, Box::new(currencies))
        .registe_module(1, Box::new(key_pair));

    if let Err(err) = bus.transition(0, "Starting".to_string()) {
        panic!("framework error: {:?}", err);
    }
}
