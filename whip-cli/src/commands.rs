use prettytable::Table;
use std::path::{PathBuf, MAIN_SEPARATOR};
use tokio::fs;

use clap::Subcommand;
use indicatif::{ProgressBar, ProgressStyle};
use sqlx::SqlitePool;
use whip_core::{download::DownloadTask, downloader::Downloader, errors::WhipError};
use whip_persistance::models::{DownloadFilter as Df, DownloadTaskEntity, DownloadTaskRepository};

#[cfg(target_family = "windows")]
pub const TEMP_DIR: &str = ".\\temp";

#[cfg(target_family = "unix")]
pub const TEMP_DIR: &str = "./temp";

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
        #[clap(value_parser, default_value = "3", long)]
        max_retries: u8,
    },
    /// Delete a download task
    Delete {
        #[clap(value_parser)]
        id: i64,
        #[clap(required = false, takes_value = false)]
        remove_file: bool,
    },
}

pub async fn handle_delete(id: i64, remove_file: bool, db_pool: SqlitePool) -> Result<(), ()> {
    let task = match db_pool.get_task_by_id(id).await {
        Ok(task) => task,
        Err(e) => {
            eprintln!("{}", e);
            return Err(());
        }
    };

    if let Some(t) = task {
        if let Err(e) = db_pool.remove_task(id).await {
            eprintln!("{}", e);
            return Err(());
        }
        if remove_file {
            for i in 0..t.max_threads {
                let f_name = format!("{tmp}{sep}{fn}.{id}", tmp = TEMP_DIR, sep = MAIN_SEPARATOR, fn=t.file_name.to_owned(), id=i);
                if PathBuf::from(&f_name).is_file() {
                    if let Err(e) = fs::remove_file(&f_name).await {
                        eprintln!("{} : Path {}", e, f_name);
                    }
                } else {
                    break;
                }
            }

            let final_file = PathBuf::from(format!("{}{}", t.final_file_path, t.file_name));
            if final_file.is_file() {
                if let Err(e) = fs::remove_file(&final_file).await {
                    eprintln!("{} : Path {}", e, final_file.to_string_lossy().to_string());
                }
            }
        }
    } else {
        println!("No task found");
    };

    Ok(())
}

pub async fn handle_download(
    url: String,
    output_dir: PathBuf,
    max_threads: u64,
    in_memory: bool,
    pool: SqlitePool,
    max_retires: u8,
) -> Result<(), ()> {
    let download_task = match pool.get_task_by_url(&url).await {
        Ok(task) => task,
        Err(_) => None,
    };

    let mut dtask_entity: DownloadTaskEntity;

    let pbr = ProgressBar::new(100);
    pbr.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise:.green}] |{bar:40.blue/cyan}| {pos:0}% ● {binary_bytes_per_sec:.green} eta {eta:.blue}",
        )
        .unwrap()
        .progress_chars("■▪▫"),
    );

    let downloader;

    let on_progress_changed = move |p: f64| {
        if !pbr.is_finished() {
            pbr.set_position(p.floor() as u64);
        }
    };
    let on_complete = |s: String| {
        println!("\nFile downloaded successfully : {}", s);
    };
    let on_error = |e: WhipError| {
        eprintln!("\n{}", e);
    };

    if let Some(d_task) = download_task {
        if d_task.percentage_completed >= 100f64 {
            let mut path = PathBuf::new();
            path.push(&d_task.final_file_path);
            path.push(&d_task.file_name);
            if path.is_file() {
                if let Ok(metadata) = path.metadata() {
                    if metadata.len() == d_task.file_size {
                        println!(
                            "File already downloaded : {}",
                            path.to_string_lossy().to_string()
                        );
                        return Ok(());
                    }
                }
            } else {
                println!(
                    "Can't find full file : {}",
                    path.to_string_lossy().to_string()
                );
            }
        }

        println!("Resuming download : {}", d_task.file_name);

        downloader = Downloader::restore(
            d_task.percentage_completed,
            d_task.to_download_task(),
            output_dir.to_string_lossy().to_string(),
            d_task.temp_files_path.to_owned(),
            on_progress_changed,
            on_complete,
            on_error,
            in_memory,
            d_task.max_threads as u8,
            max_retires,
        );
        dtask_entity = d_task;
        dtask_entity.final_file_path = output_dir.to_string_lossy().to_string();
    } else {
        println!("Profiling Download");
        let download_task = match DownloadTask::new(url).await {
            Ok(task) => task,
            Err(e) => {
                eprintln!("{}", e);
                return Err(());
            }
        };

        match pool
            .insert_task(
                &download_task,
                TEMP_DIR.to_owned(),
                output_dir.to_string_lossy().to_string(),
                max_threads.to_string(),
            )
            .await
        {
            Err(e) => {
                eprintln!("{}", e);
                return Err(());
            }
            Ok(id) => {
                dtask_entity = pool.get_task_by_id(id as i64).await.unwrap().unwrap();
            }
        };

        println!("Starting download : {}", download_task.meta.file_name);

        match Downloader::new(
            download_task,
            output_dir.to_string_lossy().to_string(),
            TEMP_DIR.to_string(),
            on_progress_changed,
            on_complete,
            on_error,
            in_memory,
            max_threads as u8,
            max_retires,
        ) {
            Ok(t) => {
                downloader = t;
            }
            Err(e) => {
                eprintln!("{}", e);
                return Err(());
            }
        }
    }

    match downloader.download().await {
        Err(e) => {
            eprintln!("{}", e);
            return Err(());
        }
        Ok(p) => {
            dtask_entity.percentage_completed = (p / dtask_entity.file_size as f64) * 100f64;
            if let Err(e) = pool.update_task(dtask_entity).await {
                eprintln!("{}", e);
                return Err(());
            };
        }
    };

    return Ok(());
}

pub async fn handle_show_downloads(filter: DownloadFilter, pool: SqlitePool) -> Result<(), ()> {
    let df = filter.into();

    let downloads = match pool.get_tasks(df).await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("{}", e);
            return Err(());
        }
    };

    if downloads.is_empty() {
        println!("You have no downloads");
        return Ok(());
    }

    let mut table = Table::new();

    table.add_row(row![bFg->"id", bFg->"File Name", bFg->"Status"]);

    for (_, download) in downloads.iter().enumerate() {
        table.add_row(row![
            download.id,
            download.file_name,
            if download.percentage_completed >= 100f64 {
                "Completed"
            } else {
                "In Progress"
            }
        ]);
    }

    table.printstd();

    return Ok(());
}
