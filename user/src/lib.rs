mod error;
pub use error::Error;

#[macro_use]
extern crate diesel;

mod models;
mod schema;

use crate::models::*;
use crate::schema::user_store::dsl::{self, user_store};
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use actix::prelude::*;
use actix::ResponseFuture;
use chrono::NaiveDateTime;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use ewf_core::error::Error as EwfError;
use ewf_core::{Bus, Call, Event, Module, StartNotify};
use serde_json::{json, Value};
use std::fmt;

use ewf_core::async_parse_check;
use wallet_common::prepare::{ModInitialParam, ModStatus};
use wallet_common::query::QueryParam;
use wallet_common::user::UserEntity;

type LocalPool = Pool<ConnectionManager<SqliteConnection>>;

static USER_STORE_TABLE: &'static str = r#"
CREATE TABLE "user_store" (
    "uid" VARCHAR(255) NOT NULL,
    "cert" VARCHAR(255) NOT NULL,
    "last_tx_time" TIMESTAMP NOT NULL,
    "account" VARCHAR(255) NOT NULL,
    PRIMARY KEY ("uid")
  )
"#;

pub struct UserModule {
    pool: LocalPool,
    bus_addr: Option<Addr<Bus>>,
}

impl fmt::Debug for UserModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{{ {} {} }}", self.name(), self.version()))
    }
}

impl UserModule {
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
        if let Err(err) = db_conn.batch_execute(&USER_STORE_TABLE) {
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
        match user_store.limit(1).load::<UserStore>(db_conn) {
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
    fn insert(db_conn: &SqliteConnection, new_currency: &NewUserStore) -> Result<(), Error> {
        let affect_rows = diesel::replace_into(user_store)
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
    fn delete(db_conn: &SqliteConnection, uid: &str) -> Result<(), Error> {
        let affect_rows = diesel::delete(user_store.find(uid))
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

    /// 模块间接口
    /// 添加交易关联用户信息
    ///     传入交易关联用户信息
    /// 异常信息
    ///     
    fn add_user(db_conn: &SqliteConnection, user: &UserEntity) -> Result<(), Error> {
        Self::insert(
            db_conn,
            &NewUserStore {
                uid: &user.uid,
                cert: &user.cert,
                last_tx_time: &NaiveDateTime::from_timestamp(
                    user.last_tx_time / 1000,
                    (user.last_tx_time % 1000 * 1_000_000) as u32,
                ),
                account: &user.account,
            },
        )?;

        Ok(())
    }

    /// 模块对外接口
    /// 查询交易关联用户信息
    ///     传入交易关联用户UID
    /// 异常信息
    ///     UserByidNotFound 未发现该用户
    fn query_user(db_conn: &SqliteConnection, uid: String) -> Result<UserEntity, Error> {
        let user = user_store
            .find(uid)
            .first::<UserStore>(db_conn)
            .map_err(|_| Error::UserByidNotFound)?;

        let user_entity = UserEntity {
            uid: user.uid,
            cert: user.cert,
            last_tx_time: user.last_tx_time.timestamp_millis(),
            account: user.account,
        };

        Ok(user_entity)
    }

    /// 模块对外接口
    /// 分页查询交易关联用户信息
    ///     传入查询条件
    ///         order_by和asc_or_desc暂不使用
    /// 异常信息
    ///     
    fn query_user_comb(
        db_conn: &SqliteConnection,
        query_param: &QueryParam,
    ) -> Result<Vec<UserEntity>, Error> {
        let users = user_store
            .order_by(dsl::last_tx_time.desc())
            .limit(query_param.page_items as i64)
            .offset((query_param.page_items * (query_param.page_num - 1)) as i64)
            .load::<UserStore>(db_conn)
            .map_err(|err| {
                log::error!("{:?}", err);
                Error::DatabaseSelectError
            })?;

        let mut rets = Vec::<UserEntity>::new();
        for user in users {
            rets.push(UserEntity {
                uid: user.uid,
                cert: user.cert,
                last_tx_time: user.last_tx_time.timestamp_millis(),
                account: user.account,
            });
        }

        Ok(rets)
    }
}

impl Actor for UserModule {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {}
}

impl Handler<Call> for UserModule {
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
                "add_user" => {
                    let param: UserEntity = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::add_user(&db_conn, &param).map_err(|err| err.to_ewf_error())?)
                }
                "query_user" => {
                    let param: String = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::query_user(&db_conn, param).map_err(|err| err.to_ewf_error())?)
                }
                "query_user_comb" => {
                    let param: QueryParam = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::query_user_comb(&db_conn, &param).map_err(|err| err.to_ewf_error())?)
                }
                _ => return Err(EwfError::MethodNotFoundError),
            };
            Ok(resp)
        })
    }
}

impl Handler<Event> for UserModule {
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

impl Handler<StartNotify> for UserModule {
    type Result = ();
    fn handle(&mut self, msg: StartNotify, _ctx: &mut Context<Self>) -> Self::Result {
        self.bus_addr = Some(msg.addr);
    }
}

impl Module for UserModule {
    fn name(&self) -> String {
        "user".to_string()
    }

    fn version(&self) -> String {
        "0.1.0".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::user_store::dsl::user_store;

    #[test]
    fn test_user_func() {
        let pool = Pool::new(ConnectionManager::new("test.db"))
            .map_err(|_| EwfError::ModuleInstanceError)
            .unwrap();
        let db_conn = pool.get().unwrap();

        UserModule::create(&db_conn).unwrap_or(());
        diesel::delete(user_store).execute(&db_conn).unwrap();

        let exampe_user = UserEntity {
            uid: "uid_001".to_string(),
            cert: "asdasdasd".to_string(),
            last_tx_time: 1596608177111,
            account: "test-account".to_string(),
        };

        UserModule::add_user(&db_conn, &exampe_user).unwrap_or(());

        let ans = UserModule::query_user(&db_conn, exampe_user.uid).unwrap();

        assert_eq!("uid_001", ans.uid);
        assert_eq!("asdasdasd", ans.cert);
        assert_eq!(1596608177111, ans.last_tx_time);
        assert_eq!("test-account", ans.account);

        let ans = UserModule::query_user_comb(
            &db_conn,
            &QueryParam {
                page_items: 10,
                page_num: 1,
                order_by: "".to_string(),
                is_asc_order: true,
            },
        )
        .unwrap();

        assert_eq!("uid_001", ans[0].uid);
        assert_eq!("asdasdasd", ans[0].cert);
        assert_eq!(1596608177111, ans[0].last_tx_time);
        assert_eq!("test-account", ans[0].account);
    }
}
