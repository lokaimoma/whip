use std::str::FromStr;

use async_trait::async_trait;
use chrono::prelude::*;
use chrono::NaiveDate;
use sqlx::SqlitePool;
use whip_core::download::DownloadTask;

use crate::{
    errors::DatabaseError,
    models::{DownloadTaskEntity, DownloadTaskRepository},
};

#[async_trait]
impl DownloadTaskRepository for SqlitePool {
    async fn insert_task(
        &self,
        task: &DownloadTask,
        temp_files_path: String,
        final_file_path: String,
        thread_count: String,
    ) -> Result<u64, DatabaseError> {
        let content_length = task.meta.content_length as i64;
        let today = Utc::today().to_string();

        if let Ok(res) = sqlx::query!(r#"Insert Into Download_Task (file_name, file_size, file_url, supports_resume, temp_files_path, final_file_path, thread_count, percentage_completed, date_created) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)"#, task.meta.file_name, content_length, task.file_url, task.meta.supports_resume, temp_files_path, final_file_path, thread_count, task.percentage_completed, today)
            .execute(self)
            .await
        {
            return Ok(res.last_insert_rowid() as u64);
        };
        Err(DatabaseError::Operation(
            "Error inserting download task".to_string(),
        ))
    }

    async fn get_tasks(&self) -> Result<Vec<DownloadTaskEntity>, DatabaseError> {
        if let Ok(download_task_entities) = sqlx::query!(r#"SELECT * FROM Download_Task"#)
            .map(|r| DownloadTaskEntity {
                id: r.id as u64,
                file_name: r.file_name,
                file_size: r.file_size.unwrap_or(0) as u64,
                file_url: r.file_url,
                supports_resume: r.supports_resume.unwrap_or(0) > 1,
                temp_files_path: r.temp_files_path,
                final_file_path: r.final_file_path,
                thread_count: r.thread_count as u64,
                percentage_completed: r.percentage_completed.unwrap_or(0f64),
                date_created: NaiveDate::from_str(&r.date_created).unwrap(),
            })
            .fetch_all(self)
            .await
        {
            return Ok(download_task_entities);
        };
        Err(DatabaseError::Operation(
            "Error fetching download tasks from database".to_string(),
        ))
    }

    async fn get_task(&self, id: i64) -> Result<Option<DownloadTaskEntity>, DatabaseError> {
        if let Ok(download_task_entity) =
            sqlx::query!(r#"SELECT * FROM Download_Task WHERE id = ?1"#, id)
                .map(|r| DownloadTaskEntity {
                    id: r.id as u64,
                    file_name: r.file_name,
                    file_size: r.file_size.unwrap_or(0) as u64,
                    file_url: r.file_url,
                    supports_resume: r.supports_resume.unwrap_or(0) > 1,
                    temp_files_path: r.temp_files_path,
                    final_file_path: r.final_file_path,
                    thread_count: r.thread_count as u64,
                    percentage_completed: r.percentage_completed.unwrap_or(0f64),
                    date_created: NaiveDate::from_str(&r.date_created).unwrap(),
                })
                .fetch_optional(self)
                .await
        {
            return Ok(download_task_entity);
        };
        Err(DatabaseError::Operation(
            "Error fetching download task from database".to_string(),
        ))
    }

    async fn update_task(
        &self,
        task: DownloadTaskEntity,
    ) -> Result<DownloadTaskEntity, DatabaseError> {
        let id = task.id as i64;
        let file_size = task.file_size as i64;

        if sqlx::query!("UPDATE Download_Task SET file_name=?1, file_url=?2, file_size=?3, percentage_completed=?4, final_file_path=?5 WHERE id = ?6", task.file_name, task.file_url, file_size, task.percentage_completed, task.final_file_path, id).execute(self).await.is_ok() {
            return Ok(task);
        };
        Err(DatabaseError::Operation(
            "Error updating download task".to_string(),
        ))
    }
}
