use errors::DatabaseError;
use sqlx::SqlitePool;

pub mod errors;
pub mod models;
pub mod queries;

pub async fn get_database_pool(database_uri: String) -> Result<SqlitePool, DatabaseError> {
    if let Ok(pool) = SqlitePool::connect(&database_uri).await {
        if let Ok(mut conn) = pool.acquire().await {
            if let Err(e) = sqlx::migrate!().run(&mut conn).await {
                return Err(DatabaseError::Operation(e.to_string()));
            };
            return Ok(pool);
        }
        return Err(DatabaseError::Operation(
            "Error acquiring database connection".to_string(),
        ));
    }

    Err(DatabaseError::Connection(
        "Error connecting to database".to_string(),
    ))
}
