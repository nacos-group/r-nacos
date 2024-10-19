use clap::{Args, Parser, Subcommand, ValueEnum};

/// A fictional versioning CLI
#[derive(Debug, Parser)] // requires `derive` feature
#[command(name = "git")]
#[command(about = "rnacos cli", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    /// env file path
    #[arg(short, long, default_value = "")]
    pub env_file: String,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// transfer middle data to sqlite
    #[command(arg_required_else_help = true)]
    DataToSqlite {
        /// the transfer middle data file
        file: String,
        /// out to sqlite db file
        out: String,
    },
}
