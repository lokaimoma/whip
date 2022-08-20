#[macro_use]
extern crate prettytable;
use clap::Parser;
use commands::{handle_delete, handle_download, handle_show_downloads, Commands, TEMP_DIR};
use dotenv::dotenv;
use sqlx::SqlitePool;
use std::{
    path::{Path, PathBuf},
    process,
};
use tokio::fs;
use whip_persistance::{errors::DatabaseError, get_database_pool};

pub mod commands;

#[derive(Parser)]
#[clap(subcommand_required = true)]
struct Whip {
    #[clap(subcommand)]
    commands: Commands,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    if !Path::new(TEMP_DIR).is_dir() {
        if let Err(e) = fs::create_dir(TEMP_DIR).await {
            eprintln!("{}", e);
            process::exit(1);
        }
    }

    let database_url =
        dotenv::var("DATABASE_URL").expect("DATABASE_URL environment variable has to be set");

    let db_pool = match setup_database(database_url).await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    let whip = Whip::parse();
    let successful: bool;

    match whip.commands {
        Commands::ShowDownloads { filter } => {
            successful = handle_show_downloads(filter, db_pool).await.is_ok();
        }
        Commands::Download {
            url,
            output_dir,
            max_threads,
            in_memory,
            max_retries,
        } => {
            successful = handle_download(
                url,
                output_dir,
                max_threads,
                in_memory,
                db_pool,
                max_retries,
            )
            .await
            .is_ok();
        }
        Commands::Delete { id, remove_file } => {
            successful = handle_delete(id, remove_file, db_pool).await.is_ok();
        }
    }

    if !successful {
        process::exit(1);
    }
}

async fn setup_database(database_url: String) -> Result<SqlitePool, DatabaseError> {
    let db_file_path = database_url.replace("sqlite:", "");

    let db_path = PathBuf::from(db_file_path);

    if !db_path.is_file() {
        if let Err(e) = fs::File::create(db_path).await {
            return Err(DatabaseError::Operation(e.to_string()));
        };
    }

    let db_pool = get_database_pool(database_url).await?;
    Ok(db_pool)
}
