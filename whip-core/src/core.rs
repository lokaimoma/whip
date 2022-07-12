use futures_util::StreamExt;
use reqwest::header;
use std::{collections::HashMap, path};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
    sync::mpsc::{self, UnboundedSender},
    task,
};

use crate::{
    download::{DownloadPart, DownloadTask},
    info::{CompletedDownload, DownloadPartProgressInfo, Progress},
};

use super::utils::{create_download_file, create_temp_file, TEMP_DIR};

pub async fn download(download_task: DownloadTask, thread_count: u32) {
    let (tx, mut rx) = mpsc::unbounded_channel::<DownloadPartProgressInfo>();

    let parts = download_task.get_download_parts(thread_count);
    let total_parts = parts.len();

    for (index, part) in parts.into_iter().enumerate() {
        let file_name = format!(
            "{fname}.part{part}",
            fname = &download_task.meta.file_name,
            part = index
        );
        let temp_file_path = format!(
            ".{sep}{temp_dir}{sep}{fname}",
            sep = path::MAIN_SEPARATOR,
            temp_dir = TEMP_DIR,
            fname = &file_name,
        );
        let tx_copy = tx.clone();
        task::spawn(async move {
            let file = create_temp_file(file_name.as_str(), 0).await.unwrap();

            download_part(part, file, temp_file_path, index as u32, tx_copy)
                .await
                .unwrap();
        });
    }

    let mut completed_downloads: HashMap<u32, CompletedDownload> =
        HashMap::with_capacity(total_parts);

    while let Some(progress) = rx.recv().await {
        match progress {
            DownloadPartProgressInfo::InProgress(data) => {
                println!(
                    "Part {} {} / {}",
                    data.part_number, data.amount_downloaded, data.total_size
                );
            }
            DownloadPartProgressInfo::Completed(cd) => {
                completed_downloads.insert(cd.part_number, cd);
                if completed_downloads.len() >= total_parts {
                    rx.close();
                    concatenate_files(&completed_downloads, &download_task.meta.file_name)
                        .await
                        .unwrap();
                }
            }
        }
    }
    println!("Done");
}

async fn concatenate_files(
    completed_downloads: &HashMap<u32, CompletedDownload>,
    final_file_name: &str,
) -> Result<(), String> {
    if let Ok(mut final_file) = create_download_file(final_file_name).await {
        for i in 0..completed_downloads.len() {
            if let Ok(data) =
                fs::read(&completed_downloads.get(&(i as u32)).unwrap().temp_file_path).await
            {
                if let Err(e) = final_file.write_all(&data).await {
                    return Err(e.to_string());
                };
            };
        }
        return Ok(());
    }
    Err(String::from("Error performing file concatenation"))
}

/// Downloads part of a file (or whole incase of non-resumable downloads).
async fn download_part(
    part: DownloadPart,
    mut temp_file: File,
    temp_file_path: String,
    part_number: u32,
    tx: UnboundedSender<DownloadPartProgressInfo>,
) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();
    let mut response_stream = client
        .get(&part.file_url)
        .header(
            header::RANGE,
            format!(
                "bytes={start}-{end}",
                start = part.start_byte,
                end = part.end_byte
            ),
        )
        .send()
        .await?
        .bytes_stream();

    let mut amount_downloaded = 0;
    let total_size = part.end_byte - part.start_byte;

    while let Some(content) = response_stream.next().await {
        if let Ok(data) = content {
            if let Ok(_) = temp_file.write_all(&data).await {
                amount_downloaded += data.len();
                

                if let Err(_) = tx.send(DownloadPartProgressInfo::InProgress(Progress {
                    amount_downloaded: amount_downloaded as u64,
                    total_size: total_size.into(),
                    part_number,
                })) {
                    return Ok(());
                };
            };
        }
    }
    if let Err(_) = tx.send(DownloadPartProgressInfo::Completed(CompletedDownload {
        temp_file_path: temp_file_path.clone(),
        part_number,
    })) {
        return Ok(());
    };
    Ok(())
}
