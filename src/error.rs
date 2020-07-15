use alloc::string::String;

#[derive(Debug)]
pub enum Error {
    NoStateMachine,
    TransitionError,
    Other(String),
}
