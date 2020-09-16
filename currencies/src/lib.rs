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
use hex::FromHex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt;

use common_structure::digital_currency::DigitalCurrencyWrapper;
use wallet_common::currencies::{
    AddCurrencyParam, CurrencyDepositParam, CurrencyEntity, CurrencyEntityShort, CurrencyQuery,
    CurrencyStatus, CurrencyWithdrawParam, CurrencyWithdrawResult, QueryCurrencyStatisticsParam,
    UnLockCurrencyParam,
};
use wallet_common::prepare::{ModInitialParam, ModStatus};

type LocalPool = Pool<ConnectionManager<SqliteConnection>>;

static CURRENCY_STORE_TABLE: &'static str = r#"
CREATE TABLE "currency_store" (
    "id" VARCHAR(255) NOT NULL,
    "owner_uid" VARCHAR(255) NOT NULL,
    "amount" BIGINT NOT NULL,
    "currency" TEXT NOT NULL,
    "txid" VARCHAR(255) NOT NULL,
    "update_time" TIMESTAMP NOT NULL,
    "last_owner_id" VARCHAR(255) NOT NULL,
    "status" INTEGER NOT NULL,
    PRIMARY KEY ("id")
  )
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyDepositRequest {
    pub target: String,
    pub bank_num: String,
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyDepositResponse(Vec<String>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyWithdrawRequest {
    pub currency: Vec<String>,
    pub bank_num: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyConvertResponse(Vec<String>);

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
            pool: Pool::new(ConnectionManager::new(&path)).map_err(|err| {
                log::error!("{:?}", err);
                EwfError::ModuleInstanceError
            })?,
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
            log::error!("{:?}", err);
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
        let affect_rows = diesel::replace_into(currency_store)
            .values(new_currency)
            .execute(db_conn)
            .map_err(|err| {
                log::error!("{:?}", err);
                Error::DatabaseInsertError
            })?;

        if affect_rows != 1 {
            return Err(Error::DatabaseInsertError);
        }
        Ok(())
    }

    /// 删除表格式数据
    fn remove_pay_lock_currency(db_conn: &SqliteConnection, id: &str) -> Result<(), Error> {
        let _affect_rows = diesel::delete(currency_store.find(id))
            .filter(dsl::status.eq(CurrencyStatus::Lock.to_int()))
            .execute(db_conn)
            .map_err(|err| {
                log::error!("{:?}", err);
                Error::DatabaseDeleteError
            })?;

        // 有可能同库两账户互转，状态刚由可用变为待见证
        // if affect_rows != 1 {
        //     return Err(Error::DatabaseDeleteError);
        // }
        Ok(())
    }

    /// 模块对外接口
    /// 添加货币到模块
    ///     传入（货币，交易ID，交易对手方ID）
    /// 异常信息
    ///     CurrencyParamInvalid 输入货币未通过校验
    fn add_currency(db_conn: &SqliteConnection, entity: &AddCurrencyParam) -> Result<(), Error> {
        let currency = DigitalCurrencyWrapper::from_bytes(
            &Vec::<u8>::from_hex(&entity.currency_str).map_err(|_| Error::CurrencyParamInvalid)?,
        )
        .map_err(|_| Error::CurrencyParamInvalid)?;
        let id = currency.get_body().get_id_str();
        let amount = currency.get_body().get_amount();

        let now = Local::now();
        let timestamp =
            NaiveDateTime::from_timestamp(now.timestamp(), now.timestamp_subsec_millis());

        let new_currency_store = NewCurrencyStore {
            id: &id,
            owner_uid: &entity.owner_uid,
            amount: amount as i64,
            currency: &entity.currency_str,
            txid: &entity.txid,
            update_time: &timestamp,
            last_owner_id: &entity.last_owner_id,
            status: CurrencyStatus::Avail.to_int(),
        };

        Self::insert(db_conn, &new_currency_store)?;

        Ok(())
    }

    /// 模块对外接口
    /// 查找货币
    ///     传入货币ID
    /// 异常信息
    ///     CurrencyByidNotFound 货币未找到
    fn find_currency_by_id(
        db_conn: &SqliteConnection,
        quota_id: &str,
    ) -> Result<CurrencyEntity, Error> {
        let currency = currency_store
            .find(quota_id)
            .first::<CurrencyStore>(db_conn)
            .map_err(|_| Error::CurrencyByidNotFound)?;

        Self::deserialize_currency(&currency)
    }

    /// 模块对外接口
    /// 查找一组货币
    ///     传入货币ID集合
    /// 异常信息
    ///     CurrencyByidNotFound 货币未找到
    fn find_currency_by_ids(
        db_conn: &SqliteConnection,
        quota_ids: Vec<String>,
    ) -> Result<Vec<CurrencyEntity>, Error> {
        let mut ret = Vec::<CurrencyEntity>::new();

        for id in quota_ids {
            let currency = currency_store
                .find(id)
                .first::<CurrencyStore>(db_conn)
                .map_err(|_| Error::CurrencyByidNotFound)?;
            ret.push(Self::deserialize_currency(&currency)?);
        }

        Ok(ret)
    }

    /// 模块对外接口
    /// 锁定一组货币
    ///     传入货币ID集合
    /// 异常信息
    ///     CurrencyByidNotFound 货币未找到
    fn lock_currency_by_ids(
        db_conn: &SqliteConnection,
        quota_ids: Vec<String>,
    ) -> Result<Vec<CurrencyEntity>, Error> {
        let mut ret = Vec::<CurrencyEntity>::new();

        for id in quota_ids {
            // sqlite 不支持returning，先更新再查询
            let updated_row = diesel::update(
                currency_store
                    .filter(dsl::id.eq(id.clone()))
                    .filter(dsl::status.eq(CurrencyStatus::Avail.to_int())),
            )
            .set(dsl::status.eq(CurrencyStatus::Lock.to_int()))
            .execute(db_conn)
            .map_err(|_| Error::PickCurrencyError)?;

            if updated_row == 0 {
                log::error!("currencies.lock_currency error when id={}", id);
                return Err(Error::PickCurrencyError);
            }

            let currency = currency_store
                .find(id)
                .first::<CurrencyStore>(db_conn)
                .map_err(|_| Error::CurrencyByidNotFound)?;
            ret.push(Self::deserialize_currency(&currency)?);
        }

        Ok(ret)
    }

    fn deserialize_currency(currency: &CurrencyStore) -> Result<CurrencyEntity, Error> {
        let avail_currency = DigitalCurrencyWrapper::from_bytes(
            &Vec::<u8>::from_hex(&currency.currency)
                .map_err(|_| Error::DatabaseJsonDeSerializeError)?,
        )
        .map_err(|_| Error::DatabaseJsonDeSerializeError)?;

        Ok(CurrencyEntity {
            id: currency.id.clone(),
            owner_uid: currency.owner_uid.clone(),
            amount: currency.amount as u64,
            currency: avail_currency,
            currency_str: currency.currency.clone(),
            txid: currency.txid.clone(),
            update_time: currency.update_time.timestamp_millis(),
            last_owner_id: currency.last_owner_id.clone(),
            status: CurrencyStatus::from(currency.status),
        })
    }

    /// 模块对外接口
    /// 分页查询管理货币列表
    ///     传入查询条件
    ///         order_by和asc_or_desc暂不使用
    /// 异常信息
    ///     
    fn query_currency_comb(
        db_conn: &SqliteConnection,
        query: &CurrencyQuery,
    ) -> Result<Vec<CurrencyEntity>, Error> {
        let currencys = currency_store
            .filter(dsl::owner_uid.eq(query.uid.clone()))
            .order_by(dsl::amount.asc())
            .limit(query.query_param.page_items as i64)
            .offset((query.query_param.page_items * (query.query_param.page_num - 1)) as i64)
            .load::<CurrencyStore>(db_conn)
            .map_err(|_| Error::DatabaseSelectError)?;

        let mut rets = Vec::<CurrencyEntity>::new();
        for currency in currencys {
            rets.push(Self::deserialize_currency(&currency)?);
        }

        Ok(rets)
    }

    /// 模块对外接口
    /// 解锁交易锁定货币
    ///     传入锁定货币的ID集合
    /// 异常信息
    ///     CurrencyUnlockError 货币解锁失败
    fn unlock_currency(
        db_conn: &SqliteConnection,
        param: &UnLockCurrencyParam,
    ) -> Result<(), Error> {
        let mut has_error = false;
        for id in &param.ids {
            let updated_row = diesel::update(
                currency_store
                    .filter(dsl::id.eq(id))
                    .filter(dsl::status.eq(CurrencyStatus::Lock.to_int())),
            )
            .set(dsl::status.eq(CurrencyStatus::Avail.to_int()))
            .execute(db_conn)
            .map_err(|_| Error::CurrencyUnlockError)?;

            if updated_row == 0 {
                log::error!("currencies.unlock_currency error when id={}", id);
                has_error = true;
            }
        }
        if has_error {
            return Err(Error::CurrencyUnlockError);
        }
        Ok(())
    }

    /// 模块对外接口
    /// 查询货币概览信息
    ///     输入要查询的用户uid, 货币种类
    /// 输出货币简略信息，由amount大到小排序
    /// 异常信息
    ///     
    fn query_currency_statistics(
        db_conn: &SqliteConnection,
        param: &QueryCurrencyStatisticsParam,
    ) -> Result<Vec<CurrencyEntityShort>, Error> {
        let currencys = currency_store
            .filter(dsl::owner_uid.eq(param.owner_uid.clone()))
            .filter(
                dsl::status
                    .eq(if param.has_avail {
                        CurrencyStatus::Avail.to_int()
                    } else {
                        -1
                    })
                    .or(dsl::status.eq(if param.has_lock {
                        CurrencyStatus::Lock.to_int()
                    } else {
                        -1
                    })),
            )
            .order_by(dsl::amount.asc())
            .load::<CurrencyStore>(db_conn)
            .map_err(|_| Error::DatabaseSelectError)?;

        let mut rets = Vec::<CurrencyEntityShort>::new();
        for each in currencys {
            rets.push(CurrencyEntityShort {
                id: each.id,
                amount: each.amount as u64,
                status: CurrencyStatus::from(each.status),
            });
        }

        Ok(rets)
    }

    /// 模块对外接口
    /// 充值
    /// 异常信息
    async fn deposit(db_conn: &SqliteConnection, param: CurrencyDepositParam) -> Result<(), Error> {
        for currency in param.currencys {
            Self::add_currency(
                db_conn,
                &AddCurrencyParam {
                    owner_uid: param.uid.clone(),
                    currency_str: currency,
                    txid: "bank".to_string(),
                    last_owner_id: "bank".to_string(),
                },
            )?;
        }

        Ok(())
    }

    /// 模块对外接口
    /// 提现
    /// 异常信息
    async fn withdraw(
        _db_conn: &SqliteConnection,
        _param: &CurrencyWithdrawParam,
    ) -> Result<CurrencyWithdrawResult, Error> {
        let ret = Vec::<String>::new();
        Ok(CurrencyWithdrawResult { currencys: ret })
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
                "add_currency" => {
                    let param: AddCurrencyParam = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::add_currency(&db_conn, &param).map_err(|err| err.to_ewf_error())?)
                }
                "remove_pay_lock_currency" => {
                    let param: String = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::remove_pay_lock_currency(&db_conn, &param)
                        .map_err(|err| err.to_ewf_error())?)
                }
                "find_currency_by_id" => {
                    let param: String = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::find_currency_by_id(&db_conn, &param)
                        .map_err(|err| err.to_ewf_error())?)
                }
                "find_currency_by_ids" => {
                    let param: Vec<String> = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::find_currency_by_ids(&db_conn, param)
                        .map_err(|err| err.to_ewf_error())?)
                }
                "lock_currency_by_ids" => {
                    let param: Vec<String> = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::lock_currency_by_ids(&db_conn, param)
                        .map_err(|err| err.to_ewf_error())?)
                }
                "query_currency_comb" => {
                    let param: CurrencyQuery = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::query_currency_comb(&db_conn, &param)
                        .map_err(|err| err.to_ewf_error())?)
                }
                "query_currency_statistics" => {
                    let param: QueryCurrencyStatisticsParam = match serde_json::from_value(msg.args)
                    {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::query_currency_statistics(&db_conn, &param)
                        .map_err(|err| err.to_ewf_error())?)
                }
                "unlock_currency" => {
                    let param: UnLockCurrencyParam = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::unlock_currency(&db_conn, &param).map_err(|err| err.to_ewf_error())?)
                }
                "deposit" => {
                    let param: CurrencyDepositParam = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::deposit(&db_conn, param)
                        .await
                        .map_err(|err| err.to_ewf_error())?)
                }
                "withdraw" => {
                    let param: CurrencyWithdrawParam = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::withdraw(&db_conn, &param)
                        .await
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
    use hex::ToHex;

    const CURRENCY_EXAMPLE: &'static str = "0303534A8CF8A3B0A3A31CBA80C07EE6A5A1CF518B6B7588802787F13A55E32FC67E949418FCA3E9148AF8D5D7F4575A51BEB00E715824728F2E7675BE48E624E45741C980D38C75ECC1B5FB87C68D4E504B899CA1F3BB9560454F871A315B4CAA9F40000000000000003143413243363843343634424244434634354636463542314145444236464531303734424141383132363837334344334343374446303544303934383244354642000000000000003033363539414536414644353230433534433438453538453936333738423138314143443443443134413039363135303238313639364636343141313435383634431027000000000000420000000000000030333533344138434638413342304133413331434241383043303745453641354131434635313842364237353838383032373837463133413535453332464336374500000000000000000000000000000000";

    #[test]
    fn test_currencies_func() {
        let pool = Pool::new(ConnectionManager::new("test.db"))
            .map_err(|_| EwfError::ModuleInstanceError)
            .unwrap();
        let db_conn = pool.get().unwrap();

        CurrenciesModule::create(&db_conn).unwrap_or(());
        diesel::delete(currency_store).execute(&db_conn).unwrap();

        let ans = CurrenciesModule::add_currency(
            &db_conn,
            &AddCurrencyParam {
                owner_uid: "test".to_string(),
                currency_str: CURRENCY_EXAMPLE.to_string(),
                txid: "zxzxc".to_string(),
                last_owner_id: "shen".to_string(),
            },
        );
        assert_eq!(ans.is_ok(), true);

        let ans = CurrenciesModule::find_currency_by_id(
            &db_conn,
            &"1CA2C68C464BBDCF45F6F5B1AEDB6FE1074BAA8126873CD3CC7DF05D09482D5F",
        );
        assert_eq!(ans.is_ok(), true);

        let ans = ans.unwrap();
        assert_eq!("test", ans.owner_uid);
        assert_eq!(10000, ans.amount);
        assert_eq!("zxzxc", ans.txid);
        assert_eq!("shen", ans.last_owner_id);
        assert_eq!(CURRENCY_EXAMPLE, ans.currency_str);
        assert_eq!(
            CURRENCY_EXAMPLE,
            ans.currency.to_bytes().encode_hex_upper::<String>()
        );
        assert_eq!(CurrencyStatus::Avail, ans.status);
    }
}
