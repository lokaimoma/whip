use std::path::{PathBuf, MAIN_SEPARATOR};

use clap::Subcommand;
use indicatif::{ProgressBar, ProgressStyle};
use sqlx::SqlitePool;
use whip_core::{download::DownloadTask, downloader::Downloader, errors::WhipError};
use whip_persistance::models::{DownloadFilter as Df, DownloadTaskEntity, DownloadTaskRepository};

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
        println!("Percentage completed {}", d_task.percentage_completed);
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
        );
        dtask_entity = d_task;
    } else {
        println!("Profiling Download");
        let download_task = match DownloadTask::new(url).await {
            Ok(task) => task,
            Err(e) => {
                eprintln!("{}", e);
                return Err(());
            }
        };

        let temp_dir = format!(".{}temp", MAIN_SEPARATOR);

        match pool
            .insert_task(
                &download_task,
                temp_dir.to_owned(),
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
            temp_dir,
            on_progress_changed,
            on_complete,
            on_error,
            in_memory,
            max_threads as u8,
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
