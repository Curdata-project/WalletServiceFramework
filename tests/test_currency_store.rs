use rustorm::Pool;
use wallet_service_framework::storage::currency_store::CurrencyStore;

#[test]
fn test_currency_store_init() {
    let mut pool = Pool::new();

    let path = "/home/xujian/Rworkspace/WalletServiceFramework/file.db";
    let is_exists = CurrencyStore::exists("./currency_store.db");
    println!("{}", is_exists);
    let currency_store = CurrencyStore::init(&format!("sqlite://{}", path), &mut pool);
}
