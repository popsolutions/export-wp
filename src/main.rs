use authors::migrate_authors;
use health::heathcheck;
use posts::migrate_posts;
use tags::migrate_tags;

mod authors;
mod health;
mod image;
mod posts;
mod tags;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    heathcheck().await;
    migrate_authors().await;
    migrate_tags().await;
    migrate_posts().await;
}
