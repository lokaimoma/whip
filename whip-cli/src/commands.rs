use std::path::{PathBuf, MAIN_SEPARATOR};

use clap::Subcommand;
use sqlx::SqlitePool;
use whip_core::{download::DownloadTask, downloader::Downloader};
use whip_persistance::models::{DownloadFilter as Df, DownloadTaskRepository};

#[derive(clap::ValueEnum, Clone)]
pub enum DownloadFilter {
    Completed,
    InProgress,
    All,
}

impl Into<Df> for DownloadFilter {
    fn into(self) -> Df {
        match self {
            DownloadFilter::All => Df::All,
            DownloadFilter::Completed => Df::Completed,
            DownloadFilter::InProgress => Df::InProgress,
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show all the download task
    ShowDownloads {
        #[clap(value_enum, long, short, default_value = "all")]
        filter: DownloadFilter,
    },
    /// Download a file
    Download {
        #[clap(value_parser)]
        url: String,
        #[clap(value_parser)]
        output_dir: PathBuf,
        #[clap(value_parser)]
        max_threads: u64,
        /// Option to store temp files in memory or on disk
        #[clap(takes_value = false, required = false)]
        in_memory: bool,
    },
}

pub async fn handle_download(
    url: String,
    output_dir: PathBuf,
    max_threads: u64,
    in_memory: bool,
    pool: SqlitePool,
) -> Result<(), ()> {
    println!("Profiling file to download....");
    let task = match DownloadTask::new(url).await {
        Ok(task) => task,
        Err(e) => {
            eprintln!("{}", e);
            return Err(());
        }
    };

    println!("File Name : {fn}\nFile Size : {fs} bytes\n", fn=task.meta.file_name, fs=task.meta.content_length);

    println!("Creating download session");

    let temp_dir = format!(".{sep}temp", sep = MAIN_SEPARATOR);

    let task_id = match pool
        .insert_task(
            &task,
            temp_dir.clone(),
            output_dir.to_string_lossy().to_string(),
            max_threads.to_string(),
        )
        .await
    {
        Ok(id) => id,
        Err(e) => {
            eprintln!("{}", e);
            return Err(());
        }
    };

    let downloader = match Downloader::new(
        task,
        output_dir.to_string_lossy().to_string(),
        temp_dir,
        |_| {},
        |path| {
            println!("File downloaded successfully\nOutput Dir : {}", path);
        },
        |e| {
            println!("{}", e);
        },
        in_memory,
    ) {
        Ok(downloader) => downloader,
        Err(e) => {
            eprintln!("{}", e);
            return Err(());
        }
    };

    println!(
        "Download session created successfully\nInitializing download, do not close the terminal"
    );

    match downloader.download(max_threads).await {
        Ok(_) => {
            match pool.get_task(task_id as i64).await {
                Ok(t) => {
                    if let Some(mut download) = t {
                        download.percentage_completed = 100f64;
                        if let Err(e) = pool.update_task(download).await {
                            eprintln!("{}", e);
                            return Err(());
                        };
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                    return Err(());
                }
            };
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(());
        }
    };

    return Ok(());
}

pub async fn handle_show_downloads(filter: DownloadFilter, pool: SqlitePool) -> Result<(), ()> {
    let df = filter.into();

    match pool.get_tasks(df).await {
        Ok(res) => {
            if res.len() == 0 {
                println!("No download task(s) added yet.");
            } else {
                for (index, task) in res.iter().enumerate() {
                    println!(
                        "#{} {} - {}%",
                        index, task.file_name, task.percentage_completed
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(());
        }
    };
    return Ok(());
}
