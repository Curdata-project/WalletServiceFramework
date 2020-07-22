use actix::prelude::*;

#[derive(Debug)]
pub enum Error {
    NoStateMachine,
    NoModule,
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
