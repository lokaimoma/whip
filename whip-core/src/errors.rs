use std::fmt;

#[derive(Debug)]
pub enum WhipError {
    Storage(String),
    NetWork(String),
    Unknown(String),
}

impl fmt::Display for WhipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WhipError::Storage(e) => write!(f, "Storage Error : {}", e),
            WhipError::NetWork(e) => write!(f, "Network Error : {}", e),
            WhipError::Unknown(e) => write!(f, "Unknown Error : {}", e),
        }
    }
}
