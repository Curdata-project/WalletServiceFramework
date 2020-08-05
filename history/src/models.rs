use super::schema::history_store;
use chrono::NaiveDateTime;

#[derive(Insertable, Debug)]
#[table_name = "history_store"]
pub struct NewHistoryStore<'a> {
    pub uid: &'a str,
    pub txid: &'a str,
    pub trans_type: i16,
    pub oppo_uid: &'a str,
    pub occur_time: &'a NaiveDateTime,
    pub amount: i64,
    pub balance: i64,
    pub remark: &'a str,
}

#[derive(Queryable, Debug, Clone)]
pub struct HistoryStore {
    pub uid: String,
    pub txid: String,
    pub trans_type: i16,
    pub oppo_uid: String,
    pub occur_time: NaiveDateTime,
    pub amount: i64,
    pub balance: i64,
    pub remark: String,
}
