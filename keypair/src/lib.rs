mod error;
pub use error::Error;

#[macro_use]
extern crate diesel;

mod models;
mod schema;

use crate::models::*;
use crate::schema::keypair_store::dsl::{keypair_store, registered_cert};
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use asymmetric_crypto::prelude::Keypair;
use dislog_hal::Bytes;
use hex::{FromHex, ToHex};
use kv_object::sm2::{CertificateSm2, KeyPairSm2};
use serde_json::{json, Value};
use std::path::Path;
use ewf_core::error::Error as FrameworkError;
use ewf_core::{Bus, Event, Module};

static KEYPAIR_STORE_TABLE: &'static str = r#"
CREATE TABLE "keypair_store" (
    "code" VARCHAR(255) NOT NULL,
    "keypair_sm2" VARCHAR(255) NOT NULL,
    "cert" VARCHAR(255) NOT NULL,
    "registered_cert" VARCHAR(255),
    "uid" VARCHAR(255),
    "info" TEXT,
    PRIMARY KEY ("code")
  )
"#;

pub struct KeypairModule {
    db_conn: SqliteConnection,
}

impl KeypairModule {
    pub fn new(path: String) -> Result<Self, FrameworkError> {
        Ok(Self {
            db_conn: SqliteConnection::establish(&path)
                .map_err(|_| FrameworkError::ModuleInstanceError)?,
        })
    }

    fn install_db(&self) -> Result<(), Error> {
        if let Err(err) = self.db_conn.batch_execute(&KEYPAIR_STORE_TABLE) {
            if err.to_string().contains("already exists") {
                return Err(Error::DatabaseExistsInstallError);
            }
            return Err(Error::DatabaseInstallError);
        }

        Ok(())
    }

    /// 获取一条密钥记录
    pub fn get_register_info(&self) -> Result<Option<KeypairStore>, Error> {
        let results = keypair_store
            .limit(1)
            .load::<KeypairStore>(&self.db_conn)
            .map_err(|_| Error::DatabaseSelectError)?;

        if results.len() == 0 {
            return Ok(None);
        }

        Ok(Some(results[0].clone()))
    }

    pub fn has_registered(&self) -> Result<bool, Error> {
        let register_info = match self.get_register_info() {
            Ok(register_info) => register_info,
            Err(err) => {
                return Ok(false);
            }
        };

        match register_info {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    /// 加表，注册密钥并存数据
    pub fn make_sure_register(&self) -> Result<(), Error> {
        if Err(Error::DatabaseInstallError) == self.install_db() {
            return Err(Error::DatabaseInstallError);
        }

        let mut rng = common_structure::get_rng_core();
        let key_pair = KeyPairSm2::generate(&mut rng).map_err(|_| Error::KeyPairGenError)?;
        let cert = key_pair.get_certificate();

        let new_keypair = NewKeypairStore {
            code: &key_pair.0.get_code().encode_hex_upper::<String>(),
            keypair_sm2: &serde_json::to_string(&key_pair).unwrap(),
            cert: &cert.to_bytes().encode_hex_upper::<String>(),
            registered_cert: &"registered_cert",
            uid: &"uid",
            info: &"info",
        };

        use crate::schema::keypair_store;
        let affect_rows = diesel::insert_into(keypair_store)
            .values(&new_keypair)
            .execute(&self.db_conn)
            .map_err(|_| Error::DatabaseInstallError)?;

        if affect_rows != 1 {
            return Err(Error::DatabaseInstallError);
        }

        Ok(())
    }
}

impl Module for KeypairModule {
    fn event_call(&self, bus: &Bus, event: &Event) -> Result<(), FrameworkError> {
        let event: &str = &event.event;
        match event {
            "StoreInitaled" => unsafe {
                if self.has_registered().unwrap() {
                    let mut_bus = &mut *(bus as *const Bus as *mut Bus);
                    mut_bus.transition(0, "Registered".to_string())?;
                } else {
                    let mut_bus = &mut *(bus as *const Bus as *mut Bus);
                    mut_bus.transition(0, "Unregistered".to_string())?;
                }
            },
            "Unregistered" => unsafe {
                let self_ = &mut *(self as *const Self as *mut Self);
                self_.make_sure_register().unwrap();

                let mut_bus = &mut *(bus as *const Bus as *mut Bus);
                mut_bus.transition(0, "RegisterComplete".to_string())?;
            },
            // no care this event, ignore
            _ => return Ok(()),
        }

        Ok(())
    }

    fn call(&self, method: &str, _intput: Value) -> Result<Value, FrameworkError> {
        match method {
            _ => Err(FrameworkError::MethodNotFoundError),
        }
    }

    fn name(&self) -> String {
        "keypair".to_string()
    }

    fn version(&self) -> String {
        "0.1.0".to_string()
    }
}
