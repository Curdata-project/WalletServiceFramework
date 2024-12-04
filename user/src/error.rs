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

    UserByidNotFound,
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
            Error::UserByidNotFound => EwfError::JsonRpcError {
                code: 3001i64,
                msg: format!("未发现该用户"),
            },
        }
    }
}
