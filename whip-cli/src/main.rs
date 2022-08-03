#[macro_use]
extern crate prettytable;
use clap::Parser;
use commands::{handle_download, handle_show_downloads, Commands};
use dotenv::dotenv;
use std::process;
use whip_persistance::get_database_pool;

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

    let database_url =
        dotenv::var("DATABASE_URL").expect("DATABASE_URL environment variable has to be set");
    let db_pool = match get_database_pool(database_url).await {
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
        } => {
            successful = handle_download(url, output_dir, max_threads, in_memory, db_pool)
                .await
                .is_ok();
        }
    }

    if !successful {
        process::exit(1);
    }
}
