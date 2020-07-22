mod error;
pub use error::Error;

#[macro_use]
extern crate diesel;

mod models;
mod schema;

use crate::models::*;
use crate::schema::currency_store::dsl::currency_store;
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use dislog_hal::Bytes;
use hex::{FromHex, ToHex};
use serde_json::{json, Value};
use std::path::Path;
use ewf_core::error::Error as FrameworkError;
use ewf_core::{Bus, Event, Module};

static CURRENCY_STORE_TABLE: &'static str = r#"
CREATE TABLE "currency_store" (
    "id" VARCHAR(255) NOT NULL,
    "jcurrency" TEXT NOT NULL,
    "txid" VARCHAR(255) NOT NULL,
    "update_time" TIMESTAMP NOT NULL,
    "last_owner_id" VARCHAR(255) NOT NULL,
    "status" INTEGER NOT NULL,
    PRIMARY KEY ("id")
  )
"#;

pub struct CurrenciesModule {
    db_conn: SqliteConnection,
}

impl CurrenciesModule {
    pub fn new(path: String) -> Result<Self, FrameworkError> {
        Ok(Self {
            db_conn: SqliteConnection::establish(&path)
                .map_err(|_| FrameworkError::ModuleInstanceError)?,
        })
    }

    fn install_db(&self) -> Result<(), Error> {
        if let Err(err) = self.db_conn.batch_execute(&CURRENCY_STORE_TABLE) {
            if err.to_string().contains("already exists") {
                return Err(Error::DatabaseExistsInstallError);
            }
            return Err(Error::DatabaseInstallError);
        }

        Ok(())
    }

    pub fn exists_db(&self) -> bool {
        match currency_store.limit(1).load::<CurrencyStore>(&self.db_conn) {
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

pub struct CurrenciesModule {
    pub bus: &'static Bus,
}

impl Actor for CurrenciesModule {
    type Context = Context<Self>;
}

impl Handler<Call> for CurrenciesModule {
    type Result = Result<Value, FrameworkError>;
    fn handle(&mut self, _msg: Call, _ctx: &mut Context<Self>) -> Self::Result {
        match method {
            _ => Err(FrameworkError::MethodNotFoundError),
        }
    }
}

impl Handler<Event> for CurrenciesModule {
    type Result = Result<(), FrameworkError>;
    fn handle(&mut self, _msg: Event, _ctx: &mut Context<Self>) -> Self::Result {
        let event: &str = &event.event;
        match event {
            "Start" => {
                if self.exists_db() {
                    unsafe {
                        let mut_bus = &mut *(bus as *const Bus as *mut Bus);
                        mut_bus.transition(0, "StoreInitaled".to_string())?;
                    }
                } else {
                    unsafe {
                        let mut_bus = &mut *(bus as *const Bus as *mut Bus);
                        mut_bus.transition(0, "EmptyWallet".to_string())?;
                    }
                }
            }
            "StoreUninital" => unsafe {
                let self_ = &mut *(self as *const Self as *mut Self);
                self_.create().unwrap();

                let mut_bus = &mut *(bus as *const Bus as *mut Bus);
                mut_bus.transition(0, "InitalSuccess".to_string())?;
            }
            // no care this event, ignore
            _ => return Ok(()),
        }

        Ok(())
    }
}

impl Module for CurrenciesModule {
    fn name(&self) -> String {
        "currencies".to_string()
    }

    fn version(&self) -> String {
        "0.1.0".to_string()
    }
}
