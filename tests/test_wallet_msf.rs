use wallet_service_framework::wallet_mgr::Wallet;

#[test]
fn test_wallet_msf() {
    let wallet = Wallet::build("./db_file".to_string());

    for each in wallet.get_module(&"wallet_mgr") {
        each.event_call(&"Start");
    }
}
