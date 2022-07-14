use std::{collections::HashMap, io::SeekFrom, path::PathBuf, sync::Arc};

use futures::{join, AsyncReadExt, AsyncWriteExt, StreamExt};
use tokio::{
    fs,
    io::{AsyncReadExt as TokioAsyncReadExt, AsyncSeekExt, AsyncWriteExt as TokioAsyncWriteExt},
    sync::Mutex,
    task,
};

use reqwest::{header, Client};

use crate::{
    download::{DownloadPart, DownloadTask},
    errors::WhipError,
    event::{CompleteStats, Event},
    storage::{FileStorage, MemoryStorage, Storage},
};

#[derive(Debug)]
enum SessionState {
    Pause,
    Stop,
    Download,
}

/// Represents a download session
/// Download only starts when start function is called.
#[derive(Debug)]
pub struct Downloader {
    /// Current download progress
    progress: u64,
    /// Current state of the session
    state: SessionState,
    /// Directory to store the file
    pub output_dir: PathBuf,
    /// Temporary directory to store download parts (when use_in_memory_sotrage = true)
    pub temp_dir: PathBuf,
    /// Information on the file to download
    pub task: DownloadTask,
    /// Callback for getting download progress updates
    pub on_progress_change: fn(f64) -> (),
    /// Callback for getting final file path on completion
    on_complete: fn(String) -> (),
    on_error: fn(WhipError) -> (),
    /// Status of the current download session
    pub completed: bool,
    /// Use in memory storage to store each part (Takes precedence over temp_dir)
    pub use_in_memory_storage: bool,
    /// Parts that have completed successfully
    completed_downloads: HashMap<u8, CompleteStats>,
    /// Number of download parts
    parts_count: u8,
}

impl Downloader {
    /// Creates a download
    pub fn new(
        task: DownloadTask,
        output_dir: String,
        temp_dir: String,
        on_progress_change: fn(f64) -> (),
        on_complete: fn(String) -> (),
        on_error: fn(WhipError) -> (),
        use_in_memory_storage: bool,
    ) -> Result<Self, WhipError> {
        let output_path = PathBuf::from(output_dir);
        if !output_path.is_dir() {
            return Err(WhipError::Storage(
                "Output directory doesn't exist or path leads to a file".to_string(),
            ));
        }
        let temp_path = PathBuf::from(temp_dir);
        if !use_in_memory_storage && !temp_path.is_dir() {
            return Err(WhipError::Storage(
                "Temporary directory doesn't exist".to_string(),
            ));
        }
        Ok(Downloader {
            progress: 0,
            completed: false,
            output_dir: output_path,
            temp_dir: temp_path,
            task,
            on_progress_change,
            use_in_memory_storage,
            state: SessionState::Download,
            completed_downloads: HashMap::new(),
            parts_count: 0,
            on_complete,
            on_error,
        })
    }

    pub fn pause(&mut self) {
        self.state = SessionState::Pause;
    }

    pub fn resume(&mut self) {
        self.state = SessionState::Download;
    }

    pub fn stop(&mut self) {
        self.state = SessionState::Stop;
    }

    pub async fn download(mut self, thread_count: u64) -> Result<Self, WhipError> {
        let client = Arc::from(reqwest::Client::new());
        let parts = self.task.get_download_parts(thread_count);
        self.parts_count = parts.len() as u8;
        let session = Arc::from(Mutex::from(self));
        let mut join_handles = Vec::new();
        for mut p in parts.into_iter() {
            let s = session.clone();
            let c = client.clone();
            let h = task::spawn(async move {
                if let Err(e) = Downloader::download_part(&s, c, &mut p).await {
                    let mut ses = s.lock().await;
                    (ses.on_error)(e);
                    ses.completed = false;
                };
            });
            join_handles.push(h);
        }
        for j in join_handles {
            let res = join!(j);
            if let Err(e) = res.0 {
                return Err(WhipError::Unknown(e.to_string()));
            }
        }
        let ses = Arc::try_unwrap(session).unwrap();
        Ok(ses.into_inner())
    }

    async fn download_part(
        session: &Arc<Mutex<Downloader>>,
        client: Arc<Client>,
        download_part: &mut DownloadPart,
    ) -> Result<(), WhipError> {
        let mut storage = Storage::InMemory(MemoryStorage::new(
            download_part.end_byte - download_part.start_byte,
        ));

        let mut sess = session.lock().await;

        if !sess.use_in_memory_storage {
            if let Some(value) = sess.setup_file_storage(&mut storage, download_part).await {
                if download_part.start_byte >= download_part.end_byte && download_part.end_byte != 0
                {
                    sess.on_event(Event::Complete(CompleteStats {
                        storage,
                        part_id: download_part.id,
                    }))
                    .await?;
                }
                return value;
            }
        }

        let task = sess.task.clone();
        drop(sess);

        let mut req = client.get(&task.file_url);

        if task.meta.supports_resume {
            req = req.header(
                header::RANGE,
                format!(
                    "bytes={start}-{end}",
                    start = download_part.start_byte,
                    end = download_part.end_byte
                ),
            )
        }

        let response = match req.send().await {
            Ok(r) => r,
            Err(e) => return Err(WhipError::NetWork(e.to_string())),
        };

        let mut bytes_stream = response.bytes_stream();

        while let Some(data) = bytes_stream.next().await {
            if let Ok(bytes) = data {
                let bytes_length = bytes.len();
                match storage {
                    Storage::InMemory(ref mut s) => {
                        if let Err(e) = s.cursor.write_all(&bytes).await {
                            return Err(WhipError::Storage(e.to_string()));
                        }
                    }

                    Storage::File(ref mut f) => {
                        if let Err(e) = f.file.write_all(&bytes).await {
                            return Err(WhipError::Storage(e.to_string()));
                        }
                    }
                };
                if let Ok(mut s) = session.try_lock() {
                    s.on_event(Event::ProgressChanged(bytes_length as u64))
                        .await
                        .unwrap();
                }
            }
        }

        session
            .lock()
            .await
            .on_event(Event::Complete(CompleteStats {
                storage,
                part_id: download_part.id,
            }))
            .await?;

        Ok(())
    }

    async fn setup_file_storage(
        &self,
        storage: &mut Storage,
        download_part: &mut DownloadPart,
    ) -> Option<Result<(), WhipError>> {
        let mut temp_file_path = PathBuf::new();
        temp_file_path.push(&self.temp_dir);
        temp_file_path.push(format!(
            "{filename}.{part_id}",
            filename = &self.task.meta.file_name,
            part_id = download_part.id
        ));

        let mut append = false;

        if self.task.meta.supports_resume && temp_file_path.exists() {
            if let Ok(metadata) = temp_file_path.metadata() {
                if metadata.len() >= (download_part.end_byte - download_part.start_byte) {
                    return Some(Ok(()));
                }
                download_part.start_byte = metadata.len();
                append = true;
            }
        }

        let file = match fs::OpenOptions::new()
            .read(true)
            .write(!append)
            .append(append)
            .create(true)
            .open(temp_file_path)
            .await
        {
            Ok(file) => file,
            Err(_) => {
                return Some(Err(WhipError::Storage(
                    "Unable to create temporary file".to_string(),
                )));
            }
        };

        *storage = Storage::File(FileStorage { file });
        None
    }

    async fn on_event(&mut self, event: Event) -> Result<(), WhipError> {
        match event {
            Event::ProgressChanged(progress) => {
                self.progress += progress;
                (self.on_progress_change)(
                    ((self.progress as f64) / (self.task.meta.content_length as f64)) * 100f64,
                );
            }
            Event::Complete(stats) => {
                self.completed_downloads.insert(stats.part_id, stats);
                if self.completed_downloads.len() >= self.parts_count.into() {
                    let f_name = self.concatenate_files().await?;
                    (self.on_progress_change)(100f64);
                    (self.on_complete)(f_name.to_string_lossy().to_string());
                }
            }
        }
        Ok(())
    }

    async fn concatenate_files(&mut self) -> Result<PathBuf, WhipError> {
        let mut f_path = PathBuf::new();
        f_path.push(&self.output_dir);
        f_path.push(&self.task.meta.file_name);
        if let Ok(mut file) = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&f_path)
            .await
        {
            for i in 0..self.completed_downloads.len() {
                let mut buffer = Vec::new();
                match self
                    .completed_downloads
                    .get_mut(&(i as u8))
                    .unwrap()
                    .storage
                {
                    Storage::File(ref mut fs) => {
                        fs.file.seek(SeekFrom::Start(0)).await.unwrap();
                        fs.file.read_to_end(&mut buffer).await.unwrap();
                    }
                    Storage::InMemory(ref mut ms) => {
                        ms.cursor.set_position(0);
                        ms.cursor.read_to_end(&mut buffer).await.unwrap();
                    }
                }
                if !buffer.is_empty() {
                    if let Err(e) = file.write_all(&buffer).await {
                        return Err(WhipError::Storage(e.to_string()));
                    }
                }
            }
            self.completed = true;
            return Ok(f_path);
        }
        Err(WhipError::Storage(
            "Error creating download file".to_string(),
        ))
    }
}
