use super::schema::user_store;
use chrono::NaiveDateTime;
use diesel::expression::*;

#[derive(Insertable)]
#[table_name = "user_store"]
pub struct NewUserStore<'a> {
    pub uid: &'a str,
    pub account: &'a str,
    pub update_time: &'a NaiveDateTime,
}

#[derive(Queryable, Debug, Clone)]
pub struct UserStore {
    pub uid: String,
    pub account: String,
    pub update_time: NaiveDateTime,
}
