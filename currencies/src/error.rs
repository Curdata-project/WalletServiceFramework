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

    CurrencyUnlockError,
    ParamDeSerializeError,
    CurrencyByidNotFound,
}

impl Error {
    pub fn to_ewf_error(self) -> EwfError {
        EwfError::OtherError(match self {
            Error::DatabaseConnectError => "DatabaseConnectError".to_string(),
            Error::DatabaseExistsInstallError => "DatabaseExistsInstallError".to_string(),
            Error::DatabaseInstallError => "DatabaseInstallError".to_string(),
            Error::DatabaseSelectError => "DatabaseSelectError".to_string(),
            Error::DatabaseInsertError => "DatabaseInsertError".to_string(),
            Error::DatabaseDeleteError => "DatabaseDeleteError".to_string(),
            Error::DatabaseJsonDeSerializeError => "DatabaseJsonDeSerializeError".to_string(),

            Error::CurrencyUnlockError => "CurrencyUnlockError".to_string(),
            Error::ParamDeSerializeError => "ParamDeSerializeError".to_string(),
            Error::CurrencyByidNotFound => "CurrencyByidNotFound".to_string(),
        })
    }
}
