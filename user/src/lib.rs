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
use ewf_core::{Bus, Call, CallQuery, Event, Module, StartNotify};
use serde_json::{json, Value};
use std::fmt;

use wallet_common::prepare::{ModStatus, ModStatusPullParam};
use wallet_common::user::{UserEntity};

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
        if let Err(err) = db_conn.batch_execute(&USER_STORE_TABLE) {
            if err.to_string().contains("already exists") {
                return Err(Error::DatabaseExistsInstallError);
            }
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
        let affect_rows = diesel::insert_into(user_store)
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
    fn delete(db_conn: &SqliteConnection, uid: &str) -> Result<(), Error> {
        let affect_rows = diesel::delete(user_store.find(uid))
            .execute(db_conn)
            .map_err(|_| Error::DatabaseDeleteError)?;

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
    fn add_user(
        db_conn: &SqliteConnection,
        user: &UserEntity,
    ) -> Result<(), Error> {
        Self::insert(db_conn, &NewUserStore{
            uid: &user.uid,
            cert: &user.cert,
            last_tx_time: &NaiveDateTime::from_timestamp(user.last_tx_time / 1000, (user.last_tx_time % 1000) as u32),
            account: &user.account,
        })?;

        Ok(())
    }

    /// 模块对外接口
    /// 查询交易关联用户信息
    ///     传入交易关联用户UID
    /// 异常信息
    ///     
    fn query_user(
        db_conn: &SqliteConnection,
        uid: String,
    ) -> Result<UserEntity, Error> {
        let user = user_store
            .find(uid)
            .first::<UserStore>(db_conn)
            .map_err(|_| Error::UserByidNotFound)?;

        let user_entity = UserEntity{
            uid: user.uid,
            cert: user.cert,
            last_tx_time: user.last_tx_time.timestamp_millis(),
            account: user.account,
        };

        Ok(user_entity)
    }
}

impl Actor for UserModule {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {}
}

impl Handler<Call> for UserModule {
    type Result = ResponseFuture<Result<Value, EwfError>>;
    fn handle(&mut self, _msg: Call, _ctx: &mut Context<Self>) -> Self::Result {
        let pool = self.pool.clone();

        Box::pin(async move {

            Ok(Value::Null)
        })
    }
}

impl Handler<Event> for UserModule {
    type Result = ResponseFuture<Result<(), EwfError>>;
    fn handle(&mut self, _msg: Event, _ctx: &mut Context<Self>) -> Self::Result {
        let pool = self.pool.clone();
        let mod_name = self.name();
        let bus_addr = self.bus_addr.clone().unwrap();

        Box::pin(async move {
            let db_conn = pool.get().unwrap();

            let event: &str = &_msg.event;
            match event {
                "Start" => {
                    let initialed = if Self::exists_db(&db_conn) || Self::create(&db_conn).is_ok() {
                        ModStatus::InitalSuccess
                    } else {
                        ModStatus::InitalFailed
                    };

                    let prepare = bus_addr
                        .send(CallQuery {
                            module: "prepare".to_string(),
                        })
                        .await??;
                    prepare
                        .send(Call {
                            method: "inital".to_string(),
                            args: json!(ModStatusPullParam {
                                mod_name: mod_name,
                                is_prepare: initialed,
                            }),
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

impl Handler<StartNotify> for UserModule {
    type Result = ();
    fn handle(&mut self, _msg: StartNotify, _ctx: &mut Context<Self>) -> Self::Result {
        self.bus_addr = Some(_msg.addr);
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
    use chrono::Local;

    #[test]
    fn test_user_func() {
        let pool = Pool::new(ConnectionManager::new("test.db"))
            .map_err(|_| EwfError::ModuleInstanceError)
            .unwrap();
        let db_conn = pool.get().unwrap();

        UserModule::create(&db_conn).unwrap_or(());
        diesel::delete(user_store).execute(&db_conn).unwrap();

        let exampe_user = UserEntity{
            uid: "uid_001".to_string(),
            cert: "asdasdasd".to_string(),
            last_tx_time: 1596608177000,
            account: "test-account".to_string(),
        };

        UserModule::add_user(&db_conn, &exampe_user).unwrap_or(());

        let ans = UserModule::query_user(&db_conn, exampe_user.uid).unwrap();

        assert_eq!("uid_001", ans.uid);
        assert_eq!("asdasdasd", ans.cert);
        assert_eq!(1596608177000, ans.last_tx_time);
        assert_eq!("test-account", ans.account);
    }
}
