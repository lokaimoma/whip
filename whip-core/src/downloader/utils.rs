use std::path;
use tokio::fs::{self, File, OpenOptions};

pub const TEMP_DIR: &str = "temp";
pub const DOWNLOADS_DIR: &str = "downloads";

/// Creates a temp file and fills it with replacementcharacter * the size you pass it
pub async fn create_temp_file(file_name: &str, size: u32) -> Result<File, String> {
    create_temp_dir().await?;
    if let Ok(file) = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .open(format!(
            ".{sep}{temp_dir}{sep}{file_name}",
            sep = path::MAIN_SEPARATOR,
            file_name = file_name,
            temp_dir = TEMP_DIR,
        ))
        .await
    {
        file.set_len(size.into()).await.unwrap();
        return Ok(file);
    };
    Err("Error creating temp file".to_string())
}

/// Checks if a directory exists.
pub async fn check_dir_exists(dir_path: &str) -> bool {
    if fs::read_dir(dir_path).await.is_ok() {
        return true;
    }
    false
}

/// Creates download file (Similar to create temp file, optimizations will be made later)
pub async fn create_download_file(file_name: &str) -> Result<File, String> {
    if !check_dir_exists(
        format!(
            ".{sep}{dlpath}",
            sep = path::MAIN_SEPARATOR,
            dlpath = DOWNLOADS_DIR
        )
        .as_str(),
    )
    .await
    {
        if let Err(e) = fs::create_dir(DOWNLOADS_DIR).await {
            return Err(e.to_string());
        }
    };
    if let Ok(file) = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .open(format!(
            ".{sep}{dl_dir}{sep}{file_name}",
            sep = path::MAIN_SEPARATOR,
            file_name = file_name,
            dl_dir = DOWNLOADS_DIR,
        ))
        .await
    {
        return Ok(file);
    }
    Err(String::from("Error creating download file"))
}

async fn create_temp_dir() -> Result<(), String> {
    if !check_dir_exists(
        format!(
            ".{sep}{temp_dir}",
            sep = path::MAIN_SEPARATOR,
            temp_dir = TEMP_DIR
        )
        .as_str(),
    )
    .await
    {
        if let Err(e) = fs::create_dir(TEMP_DIR).await {
            return Err(e.to_string());
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::os::linux::fs::MetadataExt;

    use super::*;

    #[tokio::test]
    async fn test_create_temp_file() {
        let file_size = 56;
        let file = create_temp_file("apos.exe.part1", file_size).await.unwrap();
        assert_eq!(file.metadata().await.unwrap().st_size(), file_size.into());
    }
}
