
#[derive(Debug, PartialEq)]
pub enum Error {
    DatabaseExistsInstallError,
    DatabaseInstallError,
    DatabaseSelectError,
    DatabaseInsertError,
}