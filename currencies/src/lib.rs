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
use ewf_core::async_parse_check;
use ewf_core::error::Error as EwfError;
use ewf_core::{Bus, Call, Event, Module, StartNotify};
use hex::{FromHex, ToHex};
use serde_json::{json, Value};
use std::fmt;

use common_structure::digital_currency::DigitalCurrencyWrapper;
use common_structure::transaction::TransactionWrapper;
use wallet_common::currencies::{
    AddCurrencyParam, CurrencyEntity, CurrencyStatus, UnlockCurrencyParam,
};
use wallet_common::prepare::{ModInitialParam, ModStatus};

type LocalPool = Pool<ConnectionManager<SqliteConnection>>;

static CURRENCY_STORE_TABLE: &'static str = r#"
CREATE TABLE "currency_store" (
    "id" VARCHAR(255) NOT NULL,
    "currency" TEXT NOT NULL,
    "txid" VARCHAR(255) NOT NULL,
    "update_time" TIMESTAMP NOT NULL,
    "last_owner_id" VARCHAR(255) NOT NULL,
    "status" INTEGER NOT NULL,
    PRIMARY KEY ("id")
  )
"#;

pub struct CurrenciesModule {
    pool: LocalPool,
    bus_addr: Option<Addr<Bus>>,
}

impl fmt::Debug for CurrenciesModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{{ {} {} }}", self.name(), self.version()))
    }
}

impl CurrenciesModule {
    pub fn new(path: String) -> Result<Self, EwfError> {
        Ok(Self {
            pool: Pool::new(ConnectionManager::new(&path))
                .map_err(|_| EwfError::ModuleInstanceError)?,
            bus_addr: None,
        })
    }

    /// 安装数据表
    ///
    /// 异常信息
    ///     DatabaseExistsInstallError 表已存在导致失败，一般无需关注
    ///     DatabaseInstallError 其他原因建表失败
    fn install_db(db_conn: &SqliteConnection) -> Result<(), Error> {
        if let Err(err) = db_conn.batch_execute(&CURRENCY_STORE_TABLE) {
            if err.to_string().contains("already exists") {
                return Err(Error::DatabaseExistsInstallError);
            }
            return Err(Error::DatabaseInstallError);
        }

        Ok(())
    }

    /// 检查数据表存在与否
    fn exists_db(db_conn: &SqliteConnection) -> bool {
        match currency_store.limit(1).load::<CurrencyStore>(db_conn) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// 创建数据表
    fn create(db_conn: &SqliteConnection) -> Result<(), Error> {
        if let Err(Error::DatabaseInstallError) = Self::install_db(db_conn) {
            return Err(Error::DatabaseInstallError);
        }
        Ok(())
    }

    /// 插入表格式数据，不涉及类型转换
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

    /// 删除表格式数据
    #[allow(dead_code)]
    fn delete(db_conn: &SqliteConnection, id: &str) -> Result<(), Error> {
        let affect_rows = diesel::delete(currency_store.find(id))
            .execute(db_conn)
            .map_err(|_| Error::DatabaseDeleteError)?;

        if affect_rows != 1 {
            return Err(Error::DatabaseDeleteError);
        }
        Ok(())
    }

    /// 模块对外接口
    /// 解锁货币
    ///     传入货币发行机构新发行的货币
    /// 异常信息
    ///     CurrencyUnlockError 货币不存在或重复解锁导致失败
    fn unlock_currency(
        db_conn: &SqliteConnection,
        currency: &DigitalCurrencyWrapper,
    ) -> Result<(), Error> {
        let quota_id = currency
            .get_body()
            .get_quota_info()
            .get_body()
            .get_id()
            .encode_hex::<String>();

        let currency_str = currency.to_bytes().encode_hex::<String>();

        let affect_rows = diesel::update(
            currency_store
                .find(quota_id)
                .filter(dsl::status.eq(CurrencyStatus::Lock.to_int())),
        )
        .set((
            dsl::currency.eq(currency_str),
            dsl::status.eq(CurrencyStatus::Avail.to_int()),
        ))
        .execute(db_conn)
        .map_err(|_| Error::CurrencyUnlockError)?;

        if affect_rows != 1 {
            return Err(Error::CurrencyUnlockError);
        }
        Ok(())
    }

    /// 模块对外接口
    /// 添加货币到模块
    ///     传入（货币，交易ID，交易对手方ID）
    /// 异常信息
    ///     DatabaseInsertError 货币已存在
    fn add_currency(db_conn: &SqliteConnection, entity: &AddCurrencyParam) -> Result<(), Error> {
        let (quota_id, currency_str, txid, last_owner_id, status) = match entity {
            AddCurrencyParam::AvailEntity {
                currency,
                txid,
                last_owner_id,
            } => {
                let id = currency
                    .get_body()
                    .get_quota_info()
                    .get_body()
                    .get_id()
                    .encode_hex::<String>();
                (
                    id,
                    currency.to_bytes().encode_hex::<String>(),
                    txid,
                    last_owner_id,
                    CurrencyStatus::Avail,
                )
            }
            AddCurrencyParam::LockEntity {
                transaction,
                txid,
                last_owner_id,
            } => {
                let id = transaction
                    .get_body()
                    .get_currency()
                    .get_body()
                    .get_quota_info()
                    .get_body()
                    .get_id()
                    .encode_hex::<String>();
                (
                    id,
                    transaction.to_bytes().encode_hex::<String>(),
                    txid,
                    last_owner_id,
                    CurrencyStatus::Lock,
                )
            }
        };

        let now = Local::now();
        let timestamp =
            NaiveDateTime::from_timestamp(now.timestamp(), now.timestamp_subsec_millis());

        let new_currency_store = NewCurrencyStore {
            id: &quota_id,
            currency: &currency_str,
            txid,
            update_time: &timestamp,
            last_owner_id,
            status: status.to_int(),
        };

        Self::insert(db_conn, &new_currency_store)?;

        Ok(())
    }

    /// 模块对外接口
    /// 查找货币
    ///     传入额度控制位ID
    /// 异常信息
    ///     CurrencyByidNotFound 货币未找到
    ///     DatabaseJsonDeSerializeError 意外错误，由错误逻辑导致
    fn find_currency_by_id(
        db_conn: &SqliteConnection,
        quota_id: &str,
    ) -> Result<CurrencyEntity, Error> {
        let currency = currency_store
            .find(quota_id)
            .first::<CurrencyStore>(db_conn)
            .map_err(|_| Error::CurrencyByidNotFound)?;

        Ok(match CurrencyStatus::from(currency.status) {
            CurrencyStatus::Avail => {
                let avail_currency = DigitalCurrencyWrapper::from_bytes(
                    &Vec::<u8>::from_hex(&currency.currency)
                        .map_err(|_| Error::DatabaseJsonDeSerializeError)?,
                )
                .map_err(|_| Error::DatabaseJsonDeSerializeError)?;

                CurrencyEntity::AvailEntity {
                    id: currency.id,
                    currency: avail_currency,
                    txid: currency.txid,
                    update_time: currency.update_time.timestamp_millis(),
                    last_owner_id: currency.last_owner_id,
                }
            }
            CurrencyStatus::Lock => {
                let lock_currency = TransactionWrapper::from_bytes(
                    &Vec::<u8>::from_hex(&currency.currency)
                        .map_err(|_| Error::DatabaseJsonDeSerializeError)?,
                )
                .map_err(|_| Error::DatabaseJsonDeSerializeError)?;

                CurrencyEntity::LockEntity {
                    id: currency.id,
                    transaction: lock_currency,
                    txid: currency.txid,
                    update_time: currency.update_time.timestamp_millis(),
                    last_owner_id: currency.last_owner_id,
                }
            }
        })
    }
}

impl Actor for CurrenciesModule {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {}
}

impl Handler<Call> for CurrenciesModule {
    type Result = ResponseFuture<Result<Value, EwfError>>;
    fn handle(&mut self, msg: Call, _ctx: &mut Context<Self>) -> Self::Result {
        let pool = self.pool.clone();

        Box::pin(async move {
            let db_conn = pool.get().unwrap();

            let method: &str = &msg.method;
            let resp = match method {
                "mod_initial" => {
                    let _params: ModInitialParam =
                        async_parse_check!(msg.args, EwfError::CallParamValidFaild);

                    let initialed = if Self::exists_db(&db_conn) || Self::create(&db_conn).is_ok() {
                        ModStatus::InitalSuccess
                    } else {
                        ModStatus::InitalFailed
                    };

                    json!(initialed)
                }
                "unlock_currency" => {
                    let param: UnlockCurrencyParam = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::unlock_currency(&db_conn, &param.currency)
                        .map_err(|err| err.to_ewf_error())?)
                }
                "add_currency" => {
                    let param: AddCurrencyParam = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::add_currency(&db_conn, &param).map_err(|err| err.to_ewf_error())?)
                }
                "find_currency_by_id" => {
                    let param: String = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::find_currency_by_id(&db_conn, &param)
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
        Box::pin(async move {
            let event: &str = &_msg.event;
            match event {
                // no care this event, ignore
                _ => return Ok(()),
            }
        })
    }
}

impl Handler<StartNotify> for CurrenciesModule {
    type Result = ();
    fn handle(&mut self, msg: StartNotify, _ctx: &mut Context<Self>) -> Self::Result {
        self.bus_addr = Some(msg.addr);
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
    use crate::schema::currency_store::dsl::currency_store;

    const CURRENCY_EXAMPLE: &'static str = "0303534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e70f0af45c6106766d5c983f3942747b779406c328aede49ae4d98f6790287b9ed04e53bdde5d226a8635d0151d370fd7ba9901fccf994076946673883b273f330203534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e9fe439d0e7d635d009387b60c320780ef303c61edf613222465b1f4f86805c42a63cdb945e2845f7eaec1e5ff8dda8115852875816dc4d33bcc572607a6de3d4343372267f27b5a9b5519a86ed3efc3d7a4f2a4199a907dd1d92011e875f4b98c3b0b27e730100000a0000000000000003534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e18ee7102187a30d9aff17ba95d6a7f7994e444a841fb5bc7f3698459089d8b3603659ae6afd520c54c48e58e96378b181acd4cd14a096150281696f641a145864c";

    const TRANSACTION_EXAMPLE: &'static str = "0603659ae6afd520c54c48e58e96378b181acd4cd14a096150281696f641a145864c129d6e944d589777142973d06a81e170c2ca54ea42115970d26a9b85616c1f38ed3a73f180fb885e4ccf7f8c87bd455e71a8a59cbd893d299bcda458e28bb3f62ac50654f4f7381d394c0c110f8d3e18423c3cdd01327eb0322d1683de165ea00366ad51a3bf44ee15f4c8b278b0b695a3bfc2c56602cb647cdd77867a8ae920190303534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67eafea4a289ae687a053d11c4d9f0dc815e2ff54fe088970d3f4895020e06c7f9bc42dcc15db18188e96f5ddf0dcbc8367e7c733019c1806216be9904d2132bd610203534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67ecb65c7349b6f5922b211104e34b7e444c662db98bcbf3f3595d085c8d5255ac0e0180cdee9776829c3a0788ed96301d29501b1e16ed5ff5b5f4319a69727ad53c96b931c575aaf9591e2312e6a540d651311901fd574719b6c3fb45af7f1c92e5493be7e73010000102700000000000003534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e4777110021bb7cde155743e2cc0c60ba3561295f79a6a4b63b42d5bfd5144c1903659ae6afd520c54c48e58e96378b181acd4cd14a096150281696f641a145864c";

    #[test]
    fn test_currencies_func() {
        let pool = Pool::new(ConnectionManager::new("test.db"))
            .map_err(|_| EwfError::ModuleInstanceError)
            .unwrap();
        let db_conn = pool.get().unwrap();

        CurrenciesModule::create(&db_conn).unwrap_or(());
        diesel::delete(currency_store).execute(&db_conn).unwrap();

        let currency =
            DigitalCurrencyWrapper::from_bytes(&Vec::<u8>::from_hex(&CURRENCY_EXAMPLE).unwrap())
                .unwrap();
        let ans = CurrenciesModule::add_currency(
            &db_conn,
            &AddCurrencyParam::AvailEntity {
                currency,
                txid: "zxzxc".to_string(),
                last_owner_id: "shen".to_string(),
            },
        );
        assert_eq!(ans.is_ok(), true);

        let transaction =
            TransactionWrapper::from_bytes(&Vec::<u8>::from_hex(&TRANSACTION_EXAMPLE).unwrap())
                .unwrap();
        let ans = CurrenciesModule::add_currency(
            &db_conn,
            &AddCurrencyParam::LockEntity {
                transaction,
                txid: "zxzxc1".to_string(),
                last_owner_id: "shen1".to_string(),
            },
        );
        assert_eq!(ans.is_ok(), true);

        let ans = CurrenciesModule::find_currency_by_id(
            &db_conn,
            &"343372267f27b5a9b5519a86ed3efc3d7a4f2a4199a907dd1d92011e875f4b98",
        );
        assert_eq!(ans.is_ok(), true);
        assert!(match ans.unwrap() {
            CurrencyEntity::AvailEntity {
                id,
                currency,
                txid,
                update_time: _,
                last_owner_id,
            } => {
                assert_eq!(
                    "343372267f27b5a9b5519a86ed3efc3d7a4f2a4199a907dd1d92011e875f4b98",
                    id
                );
                assert_eq!("zxzxc", txid);
                assert_eq!("shen", last_owner_id);
                assert_eq!(CURRENCY_EXAMPLE, currency.to_bytes().encode_hex::<String>());
                true
            }
            _ => false,
        });
        let ans = CurrenciesModule::find_currency_by_id(
            &db_conn,
            &"c96b931c575aaf9591e2312e6a540d651311901fd574719b6c3fb45af7f1c92e",
        );
        assert_eq!(ans.is_ok(), true);
        assert!(match ans.unwrap() {
            CurrencyEntity::LockEntity {
                id,
                transaction,
                txid,
                update_time: _,
                last_owner_id,
            } => {
                assert_eq!(
                    "c96b931c575aaf9591e2312e6a540d651311901fd574719b6c3fb45af7f1c92e",
                    id
                );
                assert_eq!("zxzxc1", txid);
                assert_eq!("shen1", last_owner_id);
                assert_eq!(
                    TRANSACTION_EXAMPLE,
                    transaction.to_bytes().encode_hex::<String>()
                );
                true
            }
            _ => false,
        });
    }
}
