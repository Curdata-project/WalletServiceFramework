use actix::prelude::*;

#[derive(Debug)]
pub enum Error {
    NoStateMachine,
    NoModule,
    ModuleInstanceError,
    MethodNotFoundError,
    TransitionNotFound,
    CallParamValidFaild,

    JsonRpcError { code: i64, msg: String },
    OtherError(String),
    ActixError(String),
}

impl<M> From<SendError<M>> for Error
where
    M: Message + Send,
    M::Result: Send,
{
    fn from(e: SendError<M>) -> Error {
        Error::ActixError(format!("{}", e))
    }
}

impl From<MailboxError> for Error {
    fn from(e: MailboxError) -> Error {
        Error::ActixError(format!("{}", e))
    }
}

/// 慎用，仅在params转换时使用
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Error {
        Error::CallParamValidFaild
    }
}
