use alloc::string::String;

#[derive(Debug)]
pub enum Error {
    NoStateMachine,
    TransitionError,
    MethodNotFoundError,
    ModuleInstanceError,

    Other(String),
}
