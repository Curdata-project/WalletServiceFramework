use ewf_core::error::Error as EwfError;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    DatabaseConnectError,
    DatabaseExistsInstallError,
    DatabaseInstallError,
    DatabaseSelectError,
    DatabaseInsertError,
    DatabaseDeleteError,
    DatabaseJsonDeSerializeError,

    CallParamValidFaild,

    RegisterInfoLack,
    KeyPairGenError,
    SecretByidNotFound,

    HttpError(String),
    HttpResponseNotJson,
    RegisterResponseInvaild,
    UnknownSecretType,
    RegisterError(String),
    SignTransactionError,
    SecretError,
}

impl Error {
    pub fn to_ewf_error(self) -> EwfError {
        match self {
            Error::DatabaseConnectError => EwfError::OtherError("DatabaseConnectError".to_string()),
            Error::DatabaseExistsInstallError => {
                EwfError::OtherError("DatabaseExistsInstallError".to_string())
            }
            Error::DatabaseInstallError => EwfError::OtherError("DatabaseInstallError".to_string()),
            Error::DatabaseSelectError => EwfError::OtherError("DatabaseSelectError".to_string()),
            Error::DatabaseInsertError => EwfError::OtherError("DatabaseInsertError".to_string()),
            Error::DatabaseDeleteError => EwfError::OtherError("DatabaseDeleteError".to_string()),
            Error::CallParamValidFaild => EwfError::CallParamValidFaild,
            Error::HttpResponseNotJson => EwfError::OtherError("HttpResponseNotJson".to_string()),
            Error::UnknownSecretType => EwfError::OtherError("UnknownSecretType".to_string()),
            Error::RegisterResponseInvaild => {
                EwfError::OtherError("RegisterResponseInvaild".to_string())
            }
            Error::DatabaseJsonDeSerializeError => {
                EwfError::OtherError("DatabaseJsonDeSerializeError".to_string())
            }
            Error::RegisterInfoLack => EwfError::JsonRpcError {
                code: 1000i64,
                msg: "请求字段缺失".to_string(),
            },
            Error::KeyPairGenError => EwfError::JsonRpcError {
                code: 1001i64,
                msg: "密钥对生成错误".to_string(),
            },
            Error::SecretByidNotFound => EwfError::JsonRpcError {
                code: 1002i64,
                msg: "未发现可用密钥对".to_string(),
            },
            Error::HttpError(err) => EwfError::JsonRpcError {
                code: 1003i64,
                msg: format!("服务器请求失败: {}", err),
            },
            Error::RegisterError(err) => EwfError::JsonRpcError {
                code: 1004i64,
                msg: format!("注册失败: ({})", err),
            },
            Error::SignTransactionError => EwfError::JsonRpcError {
                code: 1005i64,
                msg: "签名失败".to_string(),
            },
            Error::SecretError => EwfError::JsonRpcError {
                code: 1006i64,
                msg: "密钥类型错误".to_string(),
            },
        }
    }
}
