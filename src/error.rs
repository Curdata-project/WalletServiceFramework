use alloc::string::String;

#[derive(Debug)]
pub enum Error {
    NoStateMachine,
    TransitionError,
    NotMethodCallError,

    Other(String),
}
