use super::schema::user_store;
use chrono::NaiveDateTime;

#[derive(Insertable, Debug)]
#[table_name = "user_store"]
pub struct NewUserStore<'a> {
    pub uid: &'a str,
    pub cert: &'a str,
    pub last_tx_time: &'a NaiveDateTime,
    pub account: &'a str,
}

#[derive(Queryable, Debug, Clone)]
pub struct UserStore {
    pub uid: String,
    pub cert: String,
    pub last_tx_time: NaiveDateTime,
    pub account: String,
}
