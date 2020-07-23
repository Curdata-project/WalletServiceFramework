#![feature(async_closure)]

#[macro_use]
extern crate lazy_static;

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

use actix::prelude::*;
use actix::ResponseFuture;
use chrono::prelude::Local;
use chrono::NaiveDateTime;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use dislog_hal::Bytes;
use ewf_core::error::Error as EwfError;
use ewf_core::{Bus, Call, Event, Module, Transition};
use hex::ToHex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt;

use common_structure::digital_currency::DigitalCurrencyWrapper;
use common_structure::transaction::TransactionWrapper;

type LocalPool = Pool<ConnectionManager<SqliteConnection>>;

static CURRENCY_STORE_TABLE: &'static str = r#"
CREATE TABLE "currency_store" (
    "id" VARCHAR(255) NOT NULL,
    "jcurrency" VARCHAR(1024) NOT NULL,
    "txid" VARCHAR(255) NOT NULL,
    "update_time" TIMESTAMP NOT NULL,
    "last_owner_id" VARCHAR(255) NOT NULL,
    "status" INTEGER NOT NULL,
    PRIMARY KEY ("id")
  )
"#;

pub enum CurrencyStatus {
    Avail,
    Lock,
}

impl CurrencyStatus {
    pub fn to_ewf_error(self) -> i16 {
        match self {
            CurrencyStatus::Avail => 0,
            CurrencyStatus::Lock => 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockCurrencyParam {
    currency: DigitalCurrencyWrapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddAvailCurrencyParam {
    currency: DigitalCurrencyWrapper,
    txid: String,
    last_owner_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTransactionParam {
    transaction: TransactionWrapper,
    txid: String,
    last_owner_id: String,
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
    pub fn new(path: String, bus: &'static Bus) -> Result<Self, EwfError> {
        Ok(Self {
            pool: Pool::new(ConnectionManager::new(&path))
                .map_err(|_| EwfError::ModuleInstanceError)?,
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
        if let Err(Error::DatabaseInstallError) = Self::install_db(db_conn) {
            return Err(Error::DatabaseInstallError);
        }
        Ok(())
    }

    fn insert(db_conn: &SqliteConnection, new_currency: &NewCurrencyStore) -> Result<(), Error> {
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
        let affect_rows = diesel::delete(currency_store.find(id))
            .execute(db_conn)
            .map_err(|_| Error::DatabaseDeleteError)?;

        if affect_rows != 1 {
            return Err(Error::DatabaseDeleteError);
        }
        Ok(())
    }

    fn unlock_currency(
        db_conn: &SqliteConnection,
        currency: &DigitalCurrencyWrapper,
    ) -> Result<(), Error> {
        let currency_id = currency
            .get_body()
            .get_quota_info()
            .get_body()
            .get_id()
            .encode_hex::<String>();

        let currency_str = currency.to_bytes().encode_hex::<String>();

        let affect_rows = diesel::update(
            currency_store
                .find(currency_id)
                .filter(dsl::status.eq(CurrencyStatus::Lock.to_ewf_error())),
        )
        .set((
            dsl::jcurrency.eq(currency_str),
            dsl::status.eq(CurrencyStatus::Avail.to_ewf_error()),
        ))
        .execute(db_conn)
        .map_err(|_| Error::CurrencyUnlockError)?;

        if affect_rows != 1 {
            return Err(Error::CurrencyUnlockError);
        }
        Ok(())
    }

    fn add_avail_currency(
        db_conn: &SqliteConnection,
        currency: &DigitalCurrencyWrapper,
        txid: &str,
        last_owner_id: &str,
    ) -> Result<(), Error> {
        let quota_id = currency
            .get_body()
            .get_quota_info()
            .get_body()
            .get_id()
            .encode_hex::<String>();

        let currency_str = currency.to_bytes().encode_hex::<String>();

        let timestamp = NaiveDateTime::from_timestamp(Local::now().timestamp(), 0);

        let new_currency_store = NewCurrencyStore {
            id: &quota_id,
            jcurrency: &currency_str,
            txid: &txid,
            update_time: &timestamp,
            last_owner_id,
            status: CurrencyStatus::Avail.to_ewf_error(),
        };

        Self::insert(db_conn, &new_currency_store)?;

        Ok(())
    }

    fn add_transaction(
        db_conn: &SqliteConnection,
        transaction: &TransactionWrapper,
        txid: &str,
        last_owner_id: &str,
    ) -> Result<(), Error> {
        let quota_id = transaction
            .get_body()
            .get_currency()
            .get_body()
            .get_quota_info()
            .get_body()
            .get_id()
            .encode_hex::<String>();

        let transaction_str = transaction.to_bytes().encode_hex::<String>();

        let timestamp = NaiveDateTime::from_timestamp(Local::now().timestamp(), 0);

        let new_currency_store = NewCurrencyStore {
            id: &quota_id,
            jcurrency: &transaction_str,
            txid: &txid,
            update_time: &timestamp,
            last_owner_id,
            status: CurrencyStatus::Lock.to_ewf_error(),
        };

        Self::insert(db_conn, &new_currency_store)?;

        Ok(())
    }
}

impl Actor for CurrenciesModule {
    type Context = Context<Self>;
}

impl Handler<Call> for CurrenciesModule {
    type Result = ResponseFuture<Result<Value, EwfError>>;
    fn handle(&mut self, _msg: Call, _ctx: &mut Context<Self>) -> Self::Result {
        let pool = self.pool.clone();

        Box::pin(async move {
            let db_conn = pool.get().unwrap();

            let method: &str = &_msg.method;
            let resp = match method {
                "unlock_currency" => {
                    let param: UnlockCurrencyParam = match serde_json::from_value(_msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(Error::ParamDeSerializeError.to_ewf_error()),
                    };
                    json!(Self::unlock_currency(&db_conn, &param.currency)
                        .map_err(|err| err.to_ewf_error())?)
                }
                "add_avail_currency" => {
                    let param: AddAvailCurrencyParam = match serde_json::from_value(_msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(Error::ParamDeSerializeError.to_ewf_error()),
                    };
                    json!(Self::add_avail_currency(
                        &db_conn,
                        &param.currency,
                        &param.txid,
                        &param.last_owner_id
                    )
                    .map_err(|err| err.to_ewf_error())?)
                }
                "add_transaction" => {
                    let param: AddTransactionParam = match serde_json::from_value(_msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(Error::ParamDeSerializeError.to_ewf_error()),
                    };
                    json!(Self::add_transaction(
                        &db_conn,
                        &param.transaction,
                        &param.txid,
                        &param.last_owner_id
                    )
                    .map_err(|err| err.to_ewf_error())?)
                }
                _ => return Err(EwfError::MethodNotFoundError),
            };

            Ok(resp)
        })
    }
}

impl Handler<Event> for CurrenciesModule {
    type Result = ResponseFuture<Result<(), EwfError>>;
    fn handle(&mut self, _msg: Event, _ctx: &mut Context<Self>) -> Self::Result {
        let pool = self.pool.clone();

        Box::pin(async move {
            let db_conn = pool.get().unwrap();

            let id = _msg.id;
            let bus = _msg.addr;
            let event: &str = &_msg.event;
            match event {
                "Starting" => {
                    if Self::exists_db(&db_conn) || Self::create(&db_conn).is_ok() {
                        bus.send(Transition {
                            id,
                            transition: "InitalFail".to_string(),
                        })
                        .await??;
                    }

                    bus.send(Transition {
                        id,
                        transition: "InitalSuccess".to_string(),
                    })
                    .await??;
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

#[cfg(test)]
mod tests {

    use super::*;
    use ewf_core::{Bus, Transition};
    use tokio::runtime::Runtime;

    lazy_static! {
        static ref wallet_bus: Bus = Bus::new();
    }

    #[test]
    fn test_currencies_mod() {
        let rt = Runtime::new().unwrap();

        let currencies = CurrenciesModule::new("db_data".to_string(), &wallet_bus).unwrap();

        wallet_bus.module(currencies);

        let addr = wallet_bus.start();
        rt.block_on( async move{
            addr.send(Transition {
                id: 0,
                transition: "Starting".to_string(),
            }).await.unwrap();
        });
    }
}
