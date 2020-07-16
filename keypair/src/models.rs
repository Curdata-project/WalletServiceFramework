use super::schema::keypair_store;
use diesel::expression::Expression;

#[derive(Insertable)]
#[table_name="keypair_store"]
pub struct NewKeypairStore<'a> {
    pub code: &'a str,
    pub keypair_sm2: &'a str,
    pub cert: &'a str,
    pub registered_cert: &'a str,
}

#[derive(Queryable)]
pub struct KeypairStore {
    pub code: String,
    pub keypair_sm2: String,
    pub cert: String,
    pub registered_cert: String,
}