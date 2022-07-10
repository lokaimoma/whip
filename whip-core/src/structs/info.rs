#[derive(Debug)]
pub struct Progress {
    pub amount_downloaded: u64,
    pub total_size: u64,
    pub part_number: u32,
}

pub struct CompletedDownload {
    pub part_number: u32,
    pub temp_file_path: String,
}

pub enum DownloadPartProgressInfo {
    InProgress(Progress),
    Completed(CompletedDownload),
}
