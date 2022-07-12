// use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
//use whip_core::core::download;
use whip_core::{download::DownloadTask, downloader::Downloader};

#[tokio::main]
async fn main() {
    let dt = DownloadTask::new(String::from(
        "https://github.com/lokaimoma/Bugza/archive/refs/heads/main.zip",
    ))
    .await
    .unwrap();

    let downloader = Downloader::new(
        dt,
        "./downloads".to_string(),
        "./temp".to_string(),
        |percentage_completed| println!("Progress : {}/100", percentage_completed.round()),
        |file_path| println!("Download completed : {}", file_path),
        |e| println!("{:?}", e),
        true,
    )
    .unwrap();
    let _res = downloader.download(4).await.unwrap();
}
