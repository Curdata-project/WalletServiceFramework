#[macro_use]
extern crate diesel;

mod models;
mod schema;

use crate::models::*;
use crate::schema::keypair_store::dsl::{keypair_store, registered_cert};
use diesel::sqlite::SqliteConnection;
use diesel::connection::SimpleConnection;
use diesel::prelude::*;

use wallet_service_framework::{Module, Event, Bus};
use wallet_service_framework::Error;
use std::path::Path;
use serde_json::{Value, json};
use asymmetric_crypto::prelude::Keypair;
use kv_object::sm2::{CertificateSm2, KeyPairSm2};
use dislog_hal::Bytes;
use hex::{ToHex, FromHex};


static KEYPAIR_STORE_TABLE: &'static str = r#"
CREATE TABLE "keypair_store" (
    "code" VARCHAR(255) NOT NULL,
    "keypair_sm2" VARCHAR(255) NOT NULL,
    "cert" VARCHAR(255) NOT NULL,
    "registered_cert" VARCHAR(255),
    PRIMARY KEY ("code")
  )
"#;

pub struct KeypairMgr {
    db_conn: SqliteConnection,
}

impl KeypairMgr{
    pub fn new(path: String) -> Self {
        Self{
            db_conn: SqliteConnection::establish(&path)
            .expect(&format!("Error connecting to {}", path)),
        }
    }

    pub fn crate_keypair_and_register(&self) -> Result<(), Error> {
        //println!("{:?}", results);
        
        // let mut rng = common_structure::get_rng_core();
        // let key_pair = match KeyPairSm2::generate(&mut rng) {
        //     Ok(key_pair) => key_pair,
        //     Err(_) => return Err(Error::Other("KeyPairGenError".to_string())),
        // };
        // let cert = key_pair.get_certificate();

        // let new_keypair = NewKeypairStore{
        //     code: &hex::encode_upper::<String>(&key_pair.0.get_code()),
        //     keypair_sm2: &hex::encode_upper::<String>(&key_pair.0.to_bytes()),
        //     cert: &hex::encode_upper::<String>(cert.to_bytes()),
        //     registered_cert: &"registered_cert".to_string(),
        // };

        // diesel::insert_into(keypair_store::table)
        //     .values(&new_keypair)
        //     .get_result(self.db_conn)
        //     .expect("Error saving new post");

        Ok(())
    }

    pub fn has_registered(&self) -> bool {
        self.db_conn.batch_execute(&KEYPAIR_STORE_TABLE);

        let results = keypair_store.filter(registered_cert.eq("registered_cert".to_string())).limit(1)
            .load::<KeypairStore>(&self.db_conn)
            .expect("Error loading posts");

        false
    }
}


impl Module for KeypairMgr {
    fn event_call(&self, bus: &Bus, event: &Event) -> Result<(), Error> {
        let event: &str = &event.event;
        match event {
            "StoreInitaled" => {
                unsafe{ 
                    if self.has_registered() {
                        let mut_bus = &mut * ( bus as *const Bus as * mut Bus);
                        mut_bus.transition(0, "Registered".to_string())?;
                    }
                    else{
                        let mut_bus = &mut * ( bus as *const Bus as * mut Bus);
                        mut_bus.transition(0, "Unregistered".to_string())?;
                    }

                }
            }
            "Unregistered" => {
                unsafe{ 
                    let self_ = &mut * ( self as *const Self as * mut Self);
                    self_.crate_keypair_and_register();

                    let mut_bus = &mut * ( bus as *const Bus as * mut Bus);
                    mut_bus.transition(0, "RegisterComplete".to_string())?;
                }
            }
            // no care this event, ignore
            _ => return Ok(()),
        }
        Ok(())
    }

    fn call(&self, method: &str, _intput: Value) -> Result<Value, Error> {
        match method {
            _ => Err(Error::NotMethodCallError),
        }
    }

    fn name(&self) -> String {
        "keypair".to_string()
    }

    fn version(&self) -> String {
        "0.1.0".to_string()
    }
}

