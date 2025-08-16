use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rmb-cli")]
#[command(about = "Command line interface tool for interacting with Metabase APIs")]
#[command(version)]
pub struct Cli {
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[arg(short, long, global = true)]
    pub profile: Option<String>,

    #[arg(long, global = true)]
    pub config_dir: Option<String>,

    #[arg(long, global = true, env = "RMB_API_KEY")]
    pub api_key: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Authentication commands
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    /// Configuration management
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Question management and execution
    Question {
        #[command(subcommand)]
        command: QuestionCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum AuthCommands {
    /// Login to Metabase
    Login,
    /// Logout and clear the session
    Logout,
    /// Show authentication status
    Status,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Show the current configuration
    Show,
    /// Set configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum QuestionCommands {
    /// List questions
    List {
        /// Search term
        #[arg(long)]
        search: Option<String>,
        /// Limit the number of results
        #[arg(long, default_value = "20")]
        limit: u32,
        /// Collection filter
        #[arg(long)]
        collection: Option<String>,
    },
    /// Execute a question
    Execute {
        /// Question ID
        id: u32,
        /// Parameters in key=value format
        #[arg(long, action = clap::ArgAction::Append)]
        param: Vec<String>,
        /// Limit the number of results
        #[arg(long)]
        limit: Option<u32>,
    },
}