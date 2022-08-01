use std::fmt;

#[derive(Debug)]
pub enum DatabaseError {
    Connection(String),
    Operation(String),
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseError::Connection(e) => write!(f, "{}", e),
            DatabaseError::Operation(e) => write!(f, "{}", e),
        }
    }
}
