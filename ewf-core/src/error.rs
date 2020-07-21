use actix::prelude::*;

#[derive(Debug)]
pub enum Error {
    NoStateMachine,
    OtherError(String),
    ActixError(String),
}

impl<M> From<SendError<M>> for Error where M: Message + Send, M::Result: Send {
    fn from(e: SendError<M>) -> Error {
        Error::ActixError(format!("{}", e))
    }
}
