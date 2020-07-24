use super::schema::currency_store;
use chrono::NaiveDateTime;

#[derive(Insertable, Debug)]
#[table_name = "currency_store"]
pub struct NewCurrencyStore<'a> {
    pub id: &'a str,
    pub currency: &'a str,
    pub txid: &'a str,
    pub update_time: &'a NaiveDateTime,
    pub last_owner_id: &'a str,
    pub status: i16,
}

#[derive(Queryable, Debug, Clone)]
pub struct CurrencyStore {
    pub id: String,
    pub currency: String,
    pub txid: String,
    pub update_time: NaiveDateTime,
    pub last_owner_id: String,
    pub status: i16,
}
