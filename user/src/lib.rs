mod error;
pub use error::Error;

#[macro_use]
extern crate diesel;

mod models;
mod schema;

use crate::models::*;
use crate::schema::user_store::dsl::user_store;
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use dislog_hal::Bytes;
use hex::{FromHex, ToHex};
use serde_json::{json, Value};
use std::path::Path;
use wallet_service_framework::Error as FrameworkError;
use wallet_service_framework::{Bus, Event, Module};

static USER_STORE_TABLE: &'static str = r#"
CREATE TABLE "user_store" (
    "uid" VARCHAR(255) NOT NULL,
    "account" VARCHAR(255) NOT NULL,
    "update_time" TIMESTAMP NOT NULL,
    PRIMARY KEY ("uid")
  )
"#;

pub struct UserModule {
    db_conn: SqliteConnection,
}

impl UserModule {
    pub fn new(path: String) -> Result<Self, FrameworkError> {
        Ok(Self {
            db_conn: SqliteConnection::establish(&path)
                .map_err(|_| FrameworkError::ModuleInstanceError)?,
        })
    }

    fn install_db(&self) -> Result<(), Error> {
        if let Err(err) = self.db_conn.batch_execute(&USER_STORE_TABLE) {
            if err.to_string().contains("already exists") {
                return Err(Error::DatabaseExistsInstallError);
            }
            return Err(Error::DatabaseInstallError);
        }

        Ok(())
    }

    pub fn exists_db(&self) -> bool {
        match user_store.limit(1).load::<UserStore>(&self.db_conn) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub fn create(&mut self) -> Result<(), Error> {
        if Err(Error::DatabaseInstallError) == self.install_db() {
            return Err(Error::DatabaseInstallError);
        }
        Ok(())
    }
}

impl Module for UserModule {
    fn event_call(&self, bus: &Bus, event: &Event) -> Result<(), FrameworkError> {
        let event: &str = &event.event;
        match event {
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
        "user".to_string()
    }

    fn version(&self) -> String {
        "0.1.0".to_string()
    }
}
