use async_trait::async_trait;
use whip_core::download::{DownloadMeta, DownloadTask};

use crate::errors::DatabaseError;

#[derive(Debug)]
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
    /// Maximum number of threads to use if possible
    pub max_threads: u64,
    pub percentage_completed: f64,
    pub date_created: String,
    pub content_type: String,
}

impl DownloadTaskEntity {
    pub fn to_download_task(&self) -> DownloadTask {
        DownloadTask {
            file_url: self.file_url.to_owned(),
            percentage_completed: self.percentage_completed,
            meta: DownloadMeta {
                content_length: self.file_size,
                supports_resume: self.supports_resume,
                content_type: self.content_type.to_owned(),
                file_name: self.file_name.to_owned(),
            },
        }
    }
}

pub enum DownloadFilter {
    Completed,
    InProgress,
    All,
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
    async fn get_tasks(
        &self,
        filter: DownloadFilter,
    ) -> Result<Vec<DownloadTaskEntity>, DatabaseError>;
    async fn get_task_by_id(&self, id: i64) -> Result<Option<DownloadTaskEntity>, DatabaseError>;
    async fn get_task_by_url(
        &self,
        url: &String,
    ) -> Result<Option<DownloadTaskEntity>, DatabaseError>;
    async fn update_task(
        &self,
        task: DownloadTaskEntity,
    ) -> Result<DownloadTaskEntity, DatabaseError>;
    async fn remove_task(&self, id: i64) -> Result<(), DatabaseError>;
}
