use authors::migrate_authors;
use clap::Parser;
use cli::{Cli, Commands};
use health::heathcheck;
use health::test_db_connection;
use posts::migrate_posts;
use tags::migrate_tags;

mod authors;
mod health;
mod image;
mod posts;
mod tags;
mod cli;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Cli::parse();
    match args.command {
        Commands::Authors => {
            test_db_connection().await;
            migrate_authors().await;
        },
        Commands::Connections => {
            test_db_connection().await;
        },
        Commands::Pages => {
            test_db_connection().await;
            //TODO: create migration pages
        },
        Commands::Posts => {
            test_db_connection().await;
            migrate_posts();
        },
        Commands::Tags => {
            test_db_connection().await;
            migrate_tags().await;
        }        

    }
}
