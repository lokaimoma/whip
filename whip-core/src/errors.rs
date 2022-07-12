#[derive(Debug)]
pub enum WhipError {
    FileSystem(String),
    NetWork(String),
    Unknown(String),
}
