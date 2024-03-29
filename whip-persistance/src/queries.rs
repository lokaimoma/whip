use async_trait::async_trait;
use chrono::prelude::*;
use sqlx::SqlitePool;
use whip_core::download::DownloadTask;

use crate::models::DownloadFilter;
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
        let today = Utc::today().to_string().to_lowercase().replace("utc", "");

        if let Ok(res) = sqlx::query!(r#"Insert Into Download_Task (file_name, file_size, file_url, supports_resume, temp_files_path, final_file_path, thread_count, percentage_completed, date_created, content_type) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)"#, task.meta.file_name, content_length, task.file_url, task.meta.supports_resume, temp_files_path, final_file_path, thread_count, task.percentage_completed, today, task.meta.content_type)
            .execute(self)
            .await
        {
            return Ok(res.last_insert_rowid() as u64);
        };
        Err(DatabaseError::Operation(
            "Error inserting download task".to_string(),
        ))
    }

    async fn get_tasks(
        &self,
        filter: DownloadFilter,
    ) -> Result<Vec<DownloadTaskEntity>, DatabaseError> {
        let mut upper_limit = 100f64;
        let mut lower_limit = 0;

        match filter {
            DownloadFilter::Completed => lower_limit = 100,
            DownloadFilter::InProgress => upper_limit = 99.999,
            _ => {}
        }

        if let Ok(download_task_entities) = sqlx::query!(
            r#"SELECT * FROM Download_Task WHERE percentage_completed >= ?1 and percentage_completed <= ?2"#, lower_limit, upper_limit
        )
        .map(|r| DownloadTaskEntity {
            id: r.id as u64,
            file_name: r.file_name,
            file_size: r.file_size.unwrap_or(0) as u64,
            file_url: r.file_url,
            supports_resume: r.supports_resume.unwrap_or(0) > 1,
            temp_files_path: r.temp_files_path,
            final_file_path: r.final_file_path,
            max_threads: r.thread_count as u64,
            percentage_completed: r.percentage_completed.unwrap_or(0f64),
            date_created: r.date_created,
            content_type: r.content_type.unwrap_or("".to_string())
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

    async fn get_task_by_id(&self, id: i64) -> Result<Option<DownloadTaskEntity>, DatabaseError> {
        if let Ok(download_task_entity) =
            sqlx::query!(r#"SELECT * FROM Download_Task WHERE id = ?1"#, id)
                .map(|r| DownloadTaskEntity {
                    id: r.id as u64,
                    file_name: r.file_name,
                    file_size: r.file_size.unwrap_or(0) as u64,
                    file_url: r.file_url,
                    supports_resume: r.supports_resume.unwrap_or(0) >= 1,
                    temp_files_path: r.temp_files_path,
                    final_file_path: r.final_file_path,
                    max_threads: r.thread_count as u64,
                    percentage_completed: r.percentage_completed.unwrap_or(0f64),
                    date_created: r.date_created,
                    content_type: r.content_type.unwrap_or("".to_string()),
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

    async fn get_task_by_url(
        &self,
        url: &String,
    ) -> Result<Option<DownloadTaskEntity>, DatabaseError> {
        if let Ok(download_task_entity) =
            sqlx::query!(r#"SELECT * FROM Download_Task WHERE file_url = ?1"#, url)
                .map(|r| DownloadTaskEntity {
                    id: r.id as u64,
                    file_name: r.file_name,
                    file_size: r.file_size.unwrap_or(0) as u64,
                    file_url: r.file_url,
                    supports_resume: r.supports_resume.unwrap_or(0) >= 1,
                    temp_files_path: r.temp_files_path,
                    final_file_path: r.final_file_path,
                    max_threads: r.thread_count as u64,
                    percentage_completed: r.percentage_completed.unwrap_or(0f64),
                    date_created: r.date_created,
                    content_type: r.content_type.unwrap_or("".to_string()),
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

    async fn remove_task(&self, id: i64) -> Result<(), DatabaseError> {
        if let Err(e) = sqlx::query!("DELETE FROM Download_Task WHERE id = ?1", id)
            .execute(self)
            .await
        {
            return Err(DatabaseError::Operation(e.to_string()));
        };

        Ok(())
    }
}
