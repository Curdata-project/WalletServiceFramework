use currencies::CurrenciesMgr;
use wallet_service_framework::{Machine, Bus};
use wallet_service_framework::states::WalletMachine;
use keypair::KeypairMgr;


#[test]
fn test_wallet_sm() {
    let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Info)
            .try_init();

    let path = "db_data";

    let wallet_state = WalletMachine::default();
    let mut bus = Bus::new()
        .registe_machine(Box::new(wallet_state))
        .registe_module(1, Box::new(CurrenciesMgr::new(path.to_string())))
        .registe_module(1, Box::new(KeypairMgr::new(path.to_string())));
    let r = bus.transition(0, "Starting".to_string());
    log::info!("{:?}", r);
}
