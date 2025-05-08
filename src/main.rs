use authors::migrate_authors;
use clap::Parser;
use cli::{Cli, Commands};
use health::heathcheck;
use health::test_db_connection;
use posts::migrate_posts;
use tags::migrate_tags;

mod authors;
mod cli;
mod health;
mod posts;
mod tags;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Cli::parse();
    match args.command {
        Commands::Authors => {
            let _ = test_db_connection().await;
            migrate_authors().await;
        }
        Commands::Test => {
            let _ = test_db_connection().await;
            let _ = heathcheck().await;
        }
        Commands::Pages => {
            let _ = test_db_connection().await;
            // let _ = send_page().await;
            //TODO: create migration pages
        }
        Commands::Posts => {
            let _ = test_db_connection().await;
            let _ = migrate_posts().await;
        }
        Commands::Tags => {
            let _ = test_db_connection().await;
            let _ = migrate_tags().await;
        }
    }
}
