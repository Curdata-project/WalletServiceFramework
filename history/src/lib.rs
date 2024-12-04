mod error;
pub use error::Error;

#[macro_use]
extern crate diesel;

mod models;
mod schema;

use crate::models::*;
use crate::schema::history_store::dsl::{self, history_store};
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use actix::prelude::*;
use actix::ResponseFuture;
use chrono::NaiveDateTime;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use ewf_core::async_parse_check;
use ewf_core::error::Error as EwfError;
use ewf_core::{Bus, Call, Event, Module, StartNotify};
use serde_json::{json, Value};
use std::fmt;

use wallet_common::history::{HistoryEntity, TransType};
use wallet_common::prepare::{ModInitialParam, ModStatus};
use wallet_common::query::QueryParam;

type LocalPool = Pool<ConnectionManager<SqliteConnection>>;

static HISTORY_STORE_TABLE: &'static str = r#"
CREATE TABLE "history_store" (
    "uid" VARCHAR(255) NOT NULL,
    "txid" VARCHAR(255) NOT NULL,
    "trans_type" SMALLINT NOT NULL,
    "oppo_uid" VARCHAR(255) NOT NULL,
    "occur_time" TIMESTAMP NOT NULL,
    "amount" BIGINT NOT NULL,
    "balance" BIGINT NOT NULL,
    "remark" TEXT,
    PRIMARY KEY ("txid")
  )
"#;

pub struct HistoryModule {
    pool: LocalPool,
    bus_addr: Option<Addr<Bus>>,
}

impl fmt::Debug for HistoryModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{{ {} {} }}", self.name(), self.version()))
    }
}

impl HistoryModule {
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
        if let Err(err) = db_conn.batch_execute(&HISTORY_STORE_TABLE) {
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
        match history_store.limit(1).load::<HistoryStore>(db_conn) {
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
    fn insert(db_conn: &SqliteConnection, new_currency: &NewHistoryStore) -> Result<(), Error> {
        let affect_rows = diesel::replace_into(history_store)
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
    #[allow(dead_code)]
    fn delete(db_conn: &SqliteConnection, uid: &str, txid: &str) -> Result<(), Error> {
        let affect_rows = diesel::delete(history_store.find((uid, txid)))
            .execute(db_conn)
            .map_err(|err| {
                log::error!("{:?}", err);
                Error::DatabaseDeleteError
            })?;

        if affect_rows != 1 {
            return Err(Error::DatabaseDeleteError);
        }
        Ok(())
    }

    /// 模块对外接口
    /// 增加交易历史条目
    ///     传入交易历史条目
    /// 异常信息
    ///     
    fn add_history(db_conn: &SqliteConnection, history: &HistoryEntity) -> Result<(), Error> {
        Self::insert(
            db_conn,
            &NewHistoryStore {
                uid: &history.uid,
                txid: &history.txid,
                trans_type: history.trans_type.to_int16(),
                oppo_uid: &history.oppo_uid,
                occur_time: &NaiveDateTime::from_timestamp(
                    history.occur_time / 1000,
                    (history.occur_time % 1000 * 1_000_000) as u32,
                ),
                amount: history.amount as i64,
                balance: history.balance as i64,
                remark: &history.remark,
            },
        )?;
        Ok(())
    }

    /// 模块对外接口
    /// 分页查询交易关联用户信息
    ///     传入查询条件
    ///         order_by和asc_or_desc暂不使用
    /// 异常信息
    ///     
    fn query_history_comb(
        db_conn: &SqliteConnection,
        query_param: &QueryParam,
    ) -> Result<Vec<HistoryEntity>, Error> {
        let historys = history_store
            .order_by(dsl::occur_time.desc())
            .limit(query_param.page_items as i64)
            .offset((query_param.page_items * (query_param.page_num - 1)) as i64)
            .load::<HistoryStore>(db_conn)
            .map_err(|_| Error::DatabaseSelectError)?;

        let mut rets = Vec::<HistoryEntity>::new();
        for history in historys {
            rets.push(HistoryEntity {
                uid: history.uid,
                txid: history.txid,
                trans_type: TransType::from_int16(history.trans_type),
                oppo_uid: history.oppo_uid,
                occur_time: history.occur_time.timestamp_millis(),
                amount: history.amount as u64,
                balance: history.balance as u64,
                remark: history.remark,
            });
        }

        Ok(rets)
    }
}

impl Actor for HistoryModule {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {}
}

impl Handler<Call> for HistoryModule {
    type Result = ResponseFuture<Result<Value, EwfError>>;
    fn handle(&mut self, msg: Call, _ctx: &mut Context<Self>) -> Self::Result {
        let pool = self.pool.clone();

        Box::pin(async move {
            let method: &str = &msg.method;
            let db_conn = pool.get().unwrap();

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
                "add_history" => {
                    let param: HistoryEntity = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::add_history(&db_conn, &param).map_err(|err| err.to_ewf_error())?)
                }
                "query_history_comb" => {
                    let param: QueryParam = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::query_history_comb(&db_conn, &param)
                        .map_err(|err| err.to_ewf_error())?)
                }
                _ => return Err(EwfError::MethodNotFoundError),
            };

            Ok(resp)
        })
    }
}

impl Handler<Event> for HistoryModule {
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

impl Handler<StartNotify> for HistoryModule {
    type Result = ();
    fn handle(&mut self, msg: StartNotify, _ctx: &mut Context<Self>) -> Self::Result {
        self.bus_addr = Some(msg.addr);
    }
}

impl Module for HistoryModule {
    fn name(&self) -> String {
        "history".to_string()
    }

    fn version(&self) -> String {
        "0.1.0".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::history_store::dsl::history_store;

    #[test]
    fn test_history_func() {
        let pool = Pool::new(ConnectionManager::new("test.db"))
            .map_err(|_| EwfError::ModuleInstanceError)
            .unwrap();
        let db_conn = pool.get().unwrap();

        HistoryModule::create(&db_conn).unwrap_or(());
        diesel::delete(history_store).execute(&db_conn).unwrap();
    }
}
