use super::schema::secret_store;

#[derive(Insertable, Debug)]
#[table_name = "secret_store"]
pub struct NewSecretStore<'a> {
    pub uid: &'a str,
    pub secret_type: &'a str,
    pub seed: &'a str,
    pub keypair: &'a str,
    pub cert: &'a str,
}

#[derive(Queryable, Debug, Clone)]
pub struct SecretStore {
    pub uid: String,
    pub secret_type: String,
    pub seed: String,
    pub keypair: String,
    pub cert: String,
}
