use clap::{Args, Parser, Subcommand, ValueEnum};

/// A fictional versioning CLI
#[derive(Debug, Parser)] // requires `derive` feature
#[command(name = "git")]
#[command(version, about = "rnacos cli", long_about = None)]
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
    /// sqlite to transfer middle data
    #[command(arg_required_else_help = true)]
    SqliteToData {
        /// the sqlite db file
        file: String,
        /// out to transfer middle data file
        out: String,
    },
    /// nacos mysql to transfer middle data
    #[command(arg_required_else_help = true)]
    MysqlToData {
        /// the nacos mysql db uri; example: mysql://user:password@localhost:3306/nacos
        uri: String,
        /// out to transfer middle data file
        out: String,
    },
    #[command(arg_required_else_help = true)]
    OpenapiToData {
        /// nacos auth username,default is empty
        #[arg(short, long, default_value = "")]
        username: String,
        /// nacos auth password,default is empty
        #[arg(short, long, default_value = "")]
        password: String,
        /// nacos host ip:port; example: 127.0.0.1:8848
        host: String,
        /// out to transfer middle data file
        out: String,
    },
}
