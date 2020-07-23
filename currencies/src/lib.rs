#![feature(async_closure)]

mod error;
pub use error::Error;

#[macro_use]
extern crate diesel;

mod models;
mod schema;

use crate::models::*;
use crate::schema::currency_store::dsl::{self, currency_store};
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use dislog_hal::Bytes;
use hex::{FromHex, ToHex};
use serde_json::{json, Value};
use std::path::Path;
use ewf_core::error::Error as FrameworkError;
use ewf_core::{Bus, Module, Call, Event, Transition, CallQuery};
use actix::prelude::*;
use actix::{ResponseFuture};
use diesel::r2d2::Pool;
use diesel::r2d2::ConnectionManager;
use std::fmt;

type LocalPool = Pool<ConnectionManager<SqliteConnection>>;


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


pub enum CurrencyStatus{
    Avail,
    Lock,
}

impl Into<i16> for CurrencyStatus {
    
    fn into(self) -> i16 {
        match self {
            Avail => 0,
            Lock => 1,
        }
    }
}

pub struct CurrenciesModule {
    pool: LocalPool,
    pub bus: &'static Bus,
}

impl fmt::Debug for CurrenciesModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{{ pool: ..., bus: {:?} }}", self.bus))
    }
}

impl CurrenciesModule {
    pub fn new(path: String, bus: &'static Bus) -> Result<Self, FrameworkError> {
        Ok(Self {
            pool: Pool::new(ConnectionManager::new(&path))
                .map_err(|_| FrameworkError::ModuleInstanceError)?,
            bus,
        })
    }

    fn install_db(db_conn: &SqliteConnection) -> Result<(), Error> {
        if let Err(err) = db_conn.batch_execute(&CURRENCY_STORE_TABLE) {
            if err.to_string().contains("already exists") {
                return Err(Error::DatabaseExistsInstallError);
            }
            return Err(Error::DatabaseInstallError);
        }

        Ok(())
    }

    fn exists_db(db_conn: &SqliteConnection) -> bool {
        match currency_store.limit(1).load::<CurrencyStore>(db_conn) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn create(db_conn: &SqliteConnection) -> Result<(), Error> {
        if Err(Error::DatabaseInstallError) == Self::install_db(db_conn) {
            return Err(Error::DatabaseInstallError);
        }
        Ok(())
    }

    fn insert(db_conn: &SqliteConnection, new_currency: &NewCurrencyStore) -> Result<(), Error> {
        use crate::schema::currency_store;
        let affect_rows = diesel::insert_into(currency_store)
            .values(new_currency)
            .execute(db_conn)
            .map_err(|_| Error::DatabaseInsertError)?;

        if affect_rows != 1 {
            return Err(Error::DatabaseInsertError);
        }
        Ok(())
    }

    fn delete(db_conn: &SqliteConnection, id: &str) -> Result<(), Error> {
        use crate::schema::currency_store;
        let affect_rows = diesel::delete(currency_store.find(id))
        .execute(db_conn)
        .map_err(|_| Error::DatabaseDeleteError)?;

        if affect_rows != 1 {
            return Err(Error::DatabaseDeleteError);
        }
        Ok(())
    }

    fn unlock(db_conn: &SqliteConnection, currency_id: &str) -> Result<(), Error> {
        use crate::schema::currency_store;
        let status_lock: i16 = CurrencyStatus::Lock.into();
        let status_avail: i16 = CurrencyStatus::Avail.into();
        let affect_rows = diesel::update(currency_store.find(currency_id).filter(dsl::status.eq(status_lock)))
        .set(dsl::status.eq(status_avail))
        .execute(db_conn)
        .map_err(|_| Error::CurrencyUnlockError)?;

        if affect_rows != 1 {
            return Err(Error::CurrencyUnlockError);
        }
        Ok(())
    }
}

impl Actor for CurrenciesModule {
    type Context = Context<Self>;
}

impl Handler<Call> for CurrenciesModule {
    type Result = ResponseFuture<Result<Value, FrameworkError>>;
    fn handle(&mut self, _msg: Call, _ctx: &mut Context<Self>) -> Self::Result {
        Box::pin(async move {
            let method: &str = &_msg.method;
            match method {
                _ => Err(FrameworkError::MethodNotFoundError),
            }
        })
    }
}

impl Handler<Event> for CurrenciesModule {
    type Result = ResponseFuture<Result<(), FrameworkError>>;
    fn handle(&mut self, _msg: Event, _ctx: &mut Context<Self>) -> Self::Result {
        let pool = self.pool.clone();

        Box::pin(async move {
            let db_conn = pool.get().unwrap();

            let id = _msg.id;
            let bus = _msg.addr;
            let event: &str = &_msg.event;
            match event {
                "Start" => {
                    if Self::exists_db(&db_conn) {
                        bus.send(Transition{ id, transition: "StoreInitaled".to_string() }).await;
                    } else {
                        bus.send(Transition{ id, transition: "EmptyWallet".to_string() }).await;
                    }
                }
                "StoreUninital" => {
                    Self::create(&db_conn).unwrap();

                    bus.send(Transition{ id, transition: "InitalSuccess".to_string() }).await;
                }
                // no care this event, ignore
                _ => return Ok(()),
            }

            Ok(())
        })
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