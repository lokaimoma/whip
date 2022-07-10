use whip_core::downloader::core::download;
use whip_core::structs::download::DownloadTask;

#[tokio::main]
async fn main() {
    let dt = DownloadTask::new(String::from(
        "http://95.216.22.233/pcwonderland.com/download.php?url_str=https%3A%2F%2F95.216.22.233%2FiGetintopc.com%2Fdownload.php%3Ffilename%3DPcWonderland.com_7_Zip_21_x64.rar%26expires%3D1658053555%26signature%3D1b5be7a15a5742f5d65bed327651d10b&filename=PcWonderland.com_7_Zip_21_x64.rar",
    ))
    .await
    .unwrap();

    download(dt, 8).await;
}
