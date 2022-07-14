#[derive(Debug)]
pub enum WhipError {
    Storage(String),
    NetWork(String),
    Unknown(String),
}
