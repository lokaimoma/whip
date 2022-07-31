use async_trait::async_trait;
use whip_core::download::DownloadTask;

use crate::errors::DatabaseError;

pub struct DownloadTaskEntity {
    pub id: u64,
    pub file_name: String,
    pub file_size: u64,
    pub file_url: String,
    pub supports_resume: bool,
    /// Destination for temporary file parts
    pub temp_files_path: String,
    /// Destination for final file
    pub final_file_path: String,
    /// The number of threads passed when generating the download parts. Might not be equal to
    /// the number of download parts but necessary to resume download with the same number of download parts.
    pub thread_count: u64,
    pub percentage_completed: f64,
    pub date_created: chrono::NaiveDate,
}

#[async_trait]
pub trait DownloadTaskRepository {
    async fn insert_task(
        &self,
        task: &DownloadTask,
        temp_files_path: String,
        final_file_path: String,
        thread_count: String,
    ) -> Result<u64, DatabaseError>;
    async fn get_tasks(&self) -> Result<Vec<DownloadTaskEntity>, DatabaseError>;
    async fn get_task(&self, id: i64) -> Result<Option<DownloadTaskEntity>, DatabaseError>;
    async fn update_task(
        &self,
        task: DownloadTaskEntity,
    ) -> Result<DownloadTaskEntity, DatabaseError>;
}
