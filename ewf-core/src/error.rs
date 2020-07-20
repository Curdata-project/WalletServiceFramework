#[derive(Debug)]
pub enum Error {
    NoStateMachine,
    OtherError(String),
}
