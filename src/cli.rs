use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "Killer WordPress")]
#[command(about = "Killer wordpress to export by api", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Test Connections
    Test,   
    /// Migration Authors
    Authors,
    /// Migration Tags
    Tags,
    /// Migration Posts
    Posts,
    /// Migration Pages
    Pages
}