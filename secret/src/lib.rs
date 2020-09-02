mod error;
pub use error::Error;

#[macro_use]
extern crate diesel;

mod models;
mod schema;

use crate::models::*;
use crate::schema::secret_store::dsl::{self, secret_store};
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use actix::prelude::*;
use actix::ResponseFuture;
use asymmetric_crypto::prelude::Keypair;
use common_structure::get_rng_core;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use dislog_hal::Bytes;
use ewf_core::error::Error as EwfError;
use ewf_core::{async_parse_check, call_mod_througth_bus};
use ewf_core::{Bus, Call, Event, Module, StartNotify};
use hex::{FromHex, ToHex};
use kv_object::sm2::{CertificateSm2, KeyPairSm2};
use rand::RngCore;
use serde_json::{json, Value};
use std::fmt;

use common_structure::transaction::{Transaction, TransactionWrapper};
use kv_object::kv_object::MsgType;
use kv_object::prelude::KValueObject;
use wallet_common::http_cli::reqwest_json;
use wallet_common::prepare::{ModInitialParam, ModStatus};
use wallet_common::query::QueryParam;
use wallet_common::secret::{
    CertificateEntity, KeyPairEntity, RegisterParam, RegisterRequest, RegisterResponse,
    SecretEntity, SignTransactionRequest, SignTransactionResponse,
};
use wallet_common::user::UserEntity;

type LocalPool = Pool<ConnectionManager<SqliteConnection>>;

static SECRET_STORE_TABLE: &'static str = r#"
CREATE TABLE "secret_store" (
    "uid" VARCHAR(255) NOT NULL,
    "secret_type" VARCHAR(255) NOT NULL,
    "seed" VARCHAR(255) NOT NULL,
    "keypair" TEXT NOT NULL,
    "cert" TEXT NOT NULL,
    PRIMARY KEY ("uid")
  )
"#;

pub struct SecretModule {
    pool: LocalPool,
    bus_addr: Option<Addr<Bus>>,
}

impl fmt::Debug for SecretModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{{ {} {} }}", self.name(), self.version()))
    }
}

impl SecretModule {
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
        if let Err(err) = db_conn.batch_execute(&SECRET_STORE_TABLE) {
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
        match secret_store.limit(1).load::<SecretStore>(db_conn) {
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
    fn insert(db_conn: &SqliteConnection, new_secret: &NewSecretStore) -> Result<(), Error> {
        let affect_rows = diesel::replace_into(secret_store)
            .values(new_secret)
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
    fn delete(db_conn: &SqliteConnection, id: &str) -> Result<(), Error> {
        let affect_rows = diesel::delete(secret_store.find(id))
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
    /// 生成密钥并注册
    ///     传入含用户名account和密码password的json
    ///     返回注册成功的分配的用户UID
    /// 异常信息
    ///     HttpError(...) 网络请求失败
    ///     RegisterError(...) 注册请求失败
    async fn gen_and_register(
        db_conn: &SqliteConnection,
        param: RegisterParam,
    ) -> Result<SecretEntity, Error> {
        let (seed, keypair) = Self::generate_keypair_sm2()?;
        let unregister_cert = keypair.get_certificate();

        let register_req = RegisterRequest {
            cert: unregister_cert
                .to_bytes()
                .encode_hex_upper::<String>()
                .into(),
            info: param.info,
        };

        match reqwest_json(
            &param.url,
            serde_json::to_value(register_req).unwrap(),
            param.timeout,
        )
        .await
        {
            Err(err) => return Err(Error::HttpError(err)),
            Ok(resp) => {
                if resp["code"] == json!(0) {
                    let reg_resp: RegisterResponse = serde_json::from_value(resp["data"].clone())
                        .map_err(|_| Error::RegisterResponseInvaild)?;

                    let new_secret = NewSecretStore {
                        uid: &reg_resp.uid,
                        secret_type: &"sm2",
                        seed: &seed,
                        keypair: &"",
                        cert: &reg_resp.cert,
                    };
                    Self::insert(&db_conn, &new_secret)?;

                    log::info!("wallet register success, uid {}", reg_resp.uid);

                    Self::deserialize_secret(SecretStore {
                        uid: new_secret.uid.to_string(),
                        secret_type: new_secret.secret_type.to_string(),
                        seed: new_secret.seed.to_string(),
                        keypair: new_secret.keypair.to_string(),
                        cert: new_secret.cert.to_string(),
                    })
                } else {
                    log::warn!(
                        "response from {} err_code {}: message {}",
                        param.url,
                        resp["code"],
                        resp["message"]
                    );
                    Err(Error::RegisterError(format!(
                        "{}, {}",
                        resp["code"], resp["message"]
                    )))
                }
            }
        }
    }

    /// 异常 KeyPairGenError 密钥生成失败
    fn generate_keypair_sm2() -> Result<(String, KeyPairSm2), Error> {
        let mut rng = get_rng_core();
        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed);
        match KeyPairSm2::generate_from_seed(seed) {
            Ok(keypair) => Ok((keypair.0.get_code().encode_hex::<String>(), keypair)),
            Err(_) => return Err(Error::KeyPairGenError),
        }
    }

    /// 模块对外接口
    /// 根绝uid获取密钥
    ///     传入含用户uid
    ///     返回用户对应的密钥keypair和已注册证书
    /// 异常信息
    ///     SecretByidNotFound 密钥信息未发现
    fn get_secret(db_conn: &SqliteConnection, uid: &str) -> Result<SecretEntity, Error> {
        let secret = secret_store
            .find(uid)
            .first::<SecretStore>(db_conn)
            .map_err(|_| Error::SecretByidNotFound)?;

        Self::deserialize_secret(secret)
    }

    fn cert_to_string(cert: &CertificateEntity) -> String {
        match cert {
            CertificateEntity::Sm2(cert) => cert.to_bytes().encode_hex_upper::<String>(),
        }
    }

    fn deserialize_secret(secret: SecretStore) -> Result<SecretEntity, Error> {
        let secret_type: &str = &secret.secret_type;
        Ok(match secret_type {
            "sm2" => {
                let mut seed = [0u8; 32];
                let str_seed = Vec::<u8>::from_hex(&secret.seed).expect("data incrrect");
                seed.clone_from_slice(&str_seed);

                SecretEntity {
                    uid: secret.uid,
                    secret_type: secret.secret_type,
                    keypair: KeyPairEntity::Sm2(
                        KeyPairSm2::generate_from_seed(seed).expect("data incrrect"),
                    ),
                    cert: CertificateEntity::Sm2(
                        CertificateSm2::from_bytes(
                            &Vec::<u8>::from_hex(&secret.cert).expect("data incrrect"),
                        )
                        .expect("data incrrect"),
                    ),
                }
            }
            _ => return Err(Error::UnknownSecretType),
        })
    }

    /// 模块对外接口
    /// 分页查询管理密钥信息
    ///     传入查询条件
    ///         order_by和asc_or_desc暂不使用
    /// 异常信息
    ///     
    fn query_secret_comb(
        db_conn: &SqliteConnection,
        query_param: &QueryParam,
    ) -> Result<Vec<SecretEntity>, Error> {
        let secrets = secret_store
            .order_by(dsl::uid.asc())
            .limit(query_param.page_items as i64)
            .offset((query_param.page_items * (query_param.page_num - 1)) as i64)
            .load::<SecretStore>(db_conn)
            .map_err(|_| Error::SecretByidNotFound)?;

        let mut rets = Vec::<SecretEntity>::new();
        for secret in secrets {
            rets.push(Self::deserialize_secret(secret)?);
        }

        Ok(rets)
    }

    /// 模块对外接口
    /// 加密传入的交易体
    ///     传入交易体
    ///         
    /// 异常信息
    ///     
    fn sign_transaction(
        db_conn: &SqliteConnection,
        query_param: &SignTransactionRequest,
    ) -> Result<SignTransactionResponse, Error> {
        let user_secret = Self::get_secret(db_conn, &query_param.uid)?;

        let user_keypair = match user_secret.keypair {
            KeyPairEntity::Sm2(user_keypair) => user_keypair,
        };

        let mut rng = common_structure::get_rng_core();

        let mut ret = Vec::<String>::new();
        for each in &query_param.datas {
            let mut transaction = TransactionWrapper::new(
                MsgType::Transaction,
                Transaction::new(query_param.oppo_cert.clone(), each.clone()),
            );
            transaction
                .fill_kvhead(&user_keypair, &mut rng)
                .map_err(|_| Error::SignTransactionError)?;

            ret.push(transaction.to_bytes().encode_hex::<String>());
        }

        Ok(SignTransactionResponse { datas: ret })
    }
}

impl Actor for SecretModule {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {}
}

impl Handler<Call> for SecretModule {
    type Result = ResponseFuture<Result<Value, EwfError>>;
    fn handle(&mut self, msg: Call, _ctx: &mut Context<Self>) -> Self::Result {
        let pool = self.pool.clone();
        let bus_addr = self.bus_addr.clone().unwrap();

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
                "gen_and_register" => {
                    let param: RegisterParam = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };

                    let new_secret = Self::gen_and_register(&db_conn, param.clone())
                        .await
                        .map_err(|err| err.to_ewf_error())?;

                    let new_user = UserEntity {
                        uid: new_secret.uid.clone(),
                        cert: Self::cert_to_string(&new_secret.cert),
                        last_tx_time: 0,
                        account: param.info.account,
                    };

                    call_mod_througth_bus!(bus_addr, "user", "add_user", json!(new_user));

                    // tx_conn在secret后启动，此处不检查错误
                    call_mod_througth_bus!(bus_addr, "tx_conn", "bind_listen", json!(new_user));

                    json!(new_user)
                }
                "get_secret" => {
                    let param: String = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::get_secret(&db_conn, &param).map_err(|err| err.to_ewf_error())?)
                }
                "query_secret_comb" => {
                    let param: QueryParam = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::query_secret_comb(&db_conn, &param)
                        .map_err(|err| err.to_ewf_error())?)
                }
                "sign_transaction" => {
                    let param: SignTransactionRequest = match serde_json::from_value(msg.args) {
                        Ok(param) => param,
                        Err(_) => return Err(EwfError::CallParamValidFaild),
                    };
                    json!(Self::sign_transaction(&db_conn, &param)
                        .map_err(|err| err.to_ewf_error())?)
                }
                _ => return Err(EwfError::MethodNotFoundError),
            };

            Ok(resp)
        })
    }
}

impl Handler<Event> for SecretModule {
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

impl Handler<StartNotify> for SecretModule {
    type Result = ();
    fn handle(&mut self, msg: StartNotify, _ctx: &mut Context<Self>) -> Self::Result {
        self.bus_addr = Some(msg.addr);
    }
}

impl Module for SecretModule {
    fn name(&self) -> String {
        "secret".to_string()
    }

    fn version(&self) -> String {
        "0.1.0".to_string()
    }
}
