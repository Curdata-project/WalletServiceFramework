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
            Error::DatabaseJsonDeSerializeError => {
                EwfError::OtherError("DatabaseJsonDeSerializeError".to_string())
            }
            Error::CallParamValidFaild => EwfError::CallParamValidFaild,
            Error::RegisterInfoLack => EwfError::OtherError("RegisterInfoLack".to_string()),
            Error::KeyPairGenError => EwfError::OtherError("KeyPairGenError".to_string()),
            Error::SecretByidNotFound => EwfError::OtherError("SecretByidNotFound".to_string()),
            Error::HttpError(err) => EwfError::OtherError(format!("HttpError({})", err)),
            Error::HttpResponseNotJson => EwfError::OtherError("HttpResponseNotJson".to_string()),
            Error::RegisterResponseInvaild => {
                EwfError::OtherError("RegisterResponseInvaild".to_string())
            }
            Error::UnknownSecretType => EwfError::OtherError("UnknownSecretType".to_string()),
            Error::RegisterError(err) => EwfError::OtherError(format!("RegisterError({})", err)),
        }
    }
}
