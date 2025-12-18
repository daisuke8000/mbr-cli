use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mbr-cli")]
#[command(about = "Command line interface tool for interacting with Metabase APIs")]
#[command(version)]
#[command(after_help = "Examples:
  mbr-cli auth login                    # Login to Metabase
  mbr-cli query --list                  # List available questions
  mbr-cli query --list --limit 10       # List first 10 questions
  mbr-cli query 123                     # Execute question ID 123
  mbr-cli query 123 --format json       # Execute and output as JSON
  mbr-cli config show                   # Show current configuration")]
pub struct Cli {
    /// Enable verbose output for debugging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Profile name to use (default: 'default')
    #[arg(short, long, global = true)]
    pub profile: Option<String>,

    /// Custom configuration directory path
    #[arg(long, global = true)]
    pub config_dir: Option<String>,

    /// Metabase API key for authentication
    #[arg(long, global = true, env = "MBR_API_KEY")]
    pub api_key: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Authentication commands (login, logout, status)
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    /// Configuration management (show, set)
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Query Metabase questions (execute or list)
    Query(QueryArgs),
}

#[derive(Subcommand, Debug)]
pub enum AuthCommands {
    /// Login to Metabase server
    #[command(after_help = "Examples:
  mbr-cli auth login                              # Interactive login
  mbr-cli auth login --username user@example.com  # Login with username")]
    Login {
        /// Username for login (uses MBR_USERNAME env var if not provided)
        #[arg(long, env = "MBR_USERNAME")]
        username: Option<String>,
        /// Password for login (uses MBR_PASSWORD env var if not provided)
        #[arg(long, env = "MBR_PASSWORD")]
        password: Option<String>,
    },
    /// Logout and clear the session
    Logout,
    /// Show current authentication status
    Status,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Show the current configuration
    Show,
    /// Set configuration values for a profile
    #[command(after_help = "Examples:
  mbr-cli config set --url http://localhost:3000
  mbr-cli config set --profile prod --url https://metabase.example.com
  mbr-cli config set --email user@example.com")]
    Set {
        /// Profile name to configure
        #[arg(long, default_value = "default")]
        profile: String,
        /// Metabase server URL
        #[arg(long, env = "MBR_URL")]
        url: Option<String>,
        /// Email address for this profile
        #[arg(long, env = "MBR_USERNAME")]
        email: Option<String>,
    },
}

/// Query arguments for executing or listing questions
#[derive(Args, Debug)]
#[command(after_help = "Examples:
  mbr-cli query --list                  # List all questions
  mbr-cli query --list --search sales   # Search questions
  mbr-cli query 123                     # Execute question 123
  mbr-cli query 123 --format json       # Output as JSON
  mbr-cli query 123 --format csv        # Output as CSV
  mbr-cli query 123 --param date=2024-01-01  # With parameters")]
pub struct QueryArgs {
    /// Question ID to execute (omit with --list to show questions)
    pub id: Option<u32>,

    /// List available questions instead of executing
    #[arg(long, short = 'l', help_heading = "List Options")]
    pub list: bool,

    /// Search term to filter questions (only with --list)
    #[arg(long, help_heading = "List Options")]
    pub search: Option<String>,

    /// Filter by collection ID (only with --list)
    #[arg(long, help_heading = "List Options")]
    pub collection: Option<String>,

    /// Query parameters in key=value format (can be repeated)
    #[arg(long, action = clap::ArgAction::Append, help_heading = "Execution Options")]
    pub param: Vec<String>,

    /// Output format: table, json, or csv
    #[arg(short, long, default_value = "table", help_heading = "Output Options")]
    pub format: String,

    /// Maximum number of results to return
    #[arg(long, default_value = "20", help_heading = "Output Options")]
    pub limit: u32,

    /// Show all results without pagination
    #[arg(long, help_heading = "Output Options")]
    pub full: bool,

    /// Disable fullscreen interactive mode
    #[arg(long, help_heading = "Display Options")]
    pub no_fullscreen: bool,

    /// Skip first N rows (0-based offset)
    #[arg(long, help_heading = "Output Options")]
    pub offset: Option<usize>,

    /// Display only specified columns (comma-separated names)
    #[arg(long, help_heading = "Output Options")]
    pub columns: Option<String>,

    /// Number of rows per page in interactive mode
    #[arg(long, default_value = "20", help_heading = "Display Options")]
    pub page_size: usize,
}
