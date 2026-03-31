use clap::{Parser, Subcommand, ValueEnum};

use crate::cli::output::OutputFormat;

/// Color output control
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorChoice {
    /// Automatically detect terminal color support
    Auto,
    /// Always output colors
    Always,
    /// Never output colors
    Never,
}

impl ColorChoice {
    /// Returns true if colors should be used, considering terminal capabilities
    pub fn should_use_colors(self) -> bool {
        match self {
            ColorChoice::Always => true,
            ColorChoice::Never => false,
            ColorChoice::Auto => atty::is(atty::Stream::Stdout),
        }
    }
}

#[derive(Parser)]
#[command(name = "mbr-cli")]
#[command(about = "Command line interface tool for interacting with Metabase APIs")]
#[command(version)]
#[command(after_help = "Examples:
  mbr-cli login                        # Login to Metabase
  mbr-cli logout                       # Logout from Metabase
  mbr-cli queries                      # List available questions
  mbr-cli queries --limit 10           # List first 10 questions
  mbr-cli run 123                      # Execute question ID 123
  mbr-cli run 123 --format json        # Execute and output as JSON
  mbr-cli status                       # Show current config and session
  mbr-cli config set-url URL           # Set Metabase server URL
  mbr-cli config validate              # Validate session and connection
  mbr-cli collections                  # List all collections
  mbr-cli databases                    # List all databases
  mbr-cli tables 1 public              # List tables in database schema
  mbr-cli -j queries                   # JSON output for any command

Environment Variables:
  MBR_USERNAME  Metabase username (for non-interactive login)
  MBR_PASSWORD  Metabase password (for non-interactive login)
  MBR_URL       Metabase server URL")]
pub struct Cli {
    /// Output all results as JSON (overrides --format)
    #[arg(short = 'j', long, global = true)]
    pub json: bool,

    /// Enable verbose output for debugging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Custom configuration directory path
    #[arg(long, global = true)]
    pub config_dir: Option<String>,

    /// Color output control (auto, always, never)
    #[arg(long, global = true, default_value = "auto", value_enum)]
    pub color: ColorChoice,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List available questions
    #[command(visible_alias = "q")]
    Queries {
        /// Search term to filter questions
        #[arg(short, long)]
        search: Option<String>,

        /// Filter by collection ID
        #[arg(long)]
        collection: Option<String>,

        /// Maximum number of results to return
        #[arg(short, long, default_value = "20")]
        limit: u32,

        /// Output format
        #[arg(short, long, default_value = "table", value_enum)]
        format: OutputFormat,
    },

    /// Execute a question by ID
    Run {
        /// Question ID to execute
        id: u32,

        /// Query parameters in key=value format (can be repeated)
        #[arg(long, action = clap::ArgAction::Append)]
        param: Vec<String>,

        /// Output format
        #[arg(short, long, default_value = "table", value_enum)]
        format: OutputFormat,

        /// Maximum number of results to return
        #[arg(long, default_value = "20")]
        limit: u32,

        /// Show all results without pagination
        #[arg(long)]
        full: bool,

        /// Disable fullscreen interactive mode
        #[arg(long)]
        no_fullscreen: bool,

        /// Skip first N rows (0-based offset)
        #[arg(long)]
        offset: Option<usize>,

        /// Number of rows per page in interactive mode
        #[arg(long, default_value = "20")]
        page_size: usize,
    },

    /// List all collections
    #[command(visible_alias = "c")]
    Collections {
        /// Output format
        #[arg(short, long, default_value = "table", value_enum)]
        format: OutputFormat,
    },

    /// List all databases
    #[command(visible_alias = "db")]
    Databases {
        /// Output format
        #[arg(short, long, default_value = "table", value_enum)]
        format: OutputFormat,
    },

    /// List tables in a database schema
    Tables {
        /// Database ID
        database_id: u32,

        /// Schema name
        #[arg(default_value = "public")]
        schema: String,

        /// Output format
        #[arg(short, long, default_value = "table", value_enum)]
        format: OutputFormat,
    },

    /// Show current configuration and session status
    Status {
        /// Output format
        #[arg(short, long, default_value = "table", value_enum)]
        format: OutputFormat,
    },

    /// Login to Metabase with username and password
    Login,

    /// Logout from Metabase (clear session)
    Logout,

    /// Configuration management
    #[command(visible_alias = "cfg")]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Set Metabase server URL
    #[command(
        name = "set-url",
        after_help = "Examples:
  mbr-cli config set-url http://localhost:3000
  mbr-cli config set-url https://metabase.example.com"
    )]
    SetUrl {
        /// Metabase server URL
        url: String,

        /// Output format
        #[arg(short, long, default_value = "table", value_enum)]
        format: OutputFormat,
    },

    /// Validate session and test connection to Metabase server
    #[command(after_help = "Examples:
  mbr-cli config validate               # Validate current session")]
    Validate {
        /// Output format
        #[arg(short, long, default_value = "table", value_enum)]
        format: OutputFormat,
    },
}
