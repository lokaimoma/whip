pub enum DatabaseError {
    Connection(String),
    Operation(String),
}
