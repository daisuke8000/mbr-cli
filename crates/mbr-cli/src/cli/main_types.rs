use clap::{Args, Parser, Subcommand, ValueEnum};

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
  mbr-cli query list                   # List available questions
  mbr-cli query list --limit 10        # List first 10 questions
  mbr-cli query run 123                # Execute question ID 123
  mbr-cli query run 123 --format json  # Execute and output as JSON
  mbr-cli query --list                 # List questions (legacy)
  mbr-cli query 123                    # Execute question (legacy)
  mbr-cli config show                  # Show current configuration
  mbr-cli config validate              # Validate session and connection
  mbr-cli collection list              # List all collections
  mbr-cli database list                # List all databases
  mbr-cli q list                       # Short alias for query list
  mbr-cli cfg show                     # Short alias for config show

Environment Variables:
  MBR_USERNAME  Metabase username (for non-interactive login)
  MBR_PASSWORD  Metabase password (for non-interactive login)
  MBR_URL       Metabase server URL")]
pub struct Cli {
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
    /// Configuration management (show, set, validate)
    #[command(visible_alias = "cfg")]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Query Metabase questions (execute or list)
    #[command(visible_alias = "q")]
    Query(Box<QueryArgs>),
    /// Manage Metabase collections
    #[command(visible_alias = "c")]
    Collection {
        #[command(subcommand)]
        command: CollectionCommands,
    },
    /// Manage Metabase databases
    #[command(visible_alias = "db")]
    Database {
        #[command(subcommand)]
        command: DatabaseCommands,
    },
    /// Login to Metabase with username and password
    Login,
    /// Logout from Metabase (clear session)
    Logout,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Show the current configuration
    Show,
    /// Set configuration values
    #[command(after_help = "Examples:
  mbr-cli config set --url http://localhost:3000
  mbr-cli config set --url https://metabase.example.com")]
    Set {
        /// Metabase server URL
        #[arg(long, env = "MBR_URL")]
        url: Option<String>,
    },
    /// Validate session and test connection to Metabase server
    #[command(after_help = "Examples:
  mbr-cli config validate               # Validate current session")]
    Validate,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CollectionCommands {
    /// List all collections
    #[command(after_help = "Examples:
  mbr-cli collection list                # List all collections
  mbr-cli collection list --format json  # Output as JSON
  mbr-cli collection list --format csv   # Output as CSV")]
    List {
        /// Output format: table, json, or csv
        #[arg(short, long, default_value = "table")]
        format: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum DatabaseCommands {
    /// List all databases
    #[command(after_help = "Examples:
  mbr-cli database list                # List all databases
  mbr-cli database list --format json  # Output as JSON
  mbr-cli database list --format csv   # Output as CSV")]
    List {
        /// Output format: table, json, or csv
        #[arg(short, long, default_value = "table")]
        format: String,
    },
}

/// Query subcommands for the new subcommand-based interface
#[derive(Subcommand, Debug, Clone)]
pub enum QueryCommand {
    /// List available questions
    #[command(
        visible_alias = "ls",
        after_help = "Examples:
  mbr-cli query list                   # List all questions
  mbr-cli query list --search sales    # Search questions
  mbr-cli query list --limit 10        # Limit results"
    )]
    List {
        /// Search term to filter questions
        #[arg(short, long)]
        search: Option<String>,

        /// Filter by collection ID
        #[arg(long)]
        collection: Option<String>,

        /// Maximum number of results to return
        #[arg(short, long, default_value = "20")]
        limit: u32,
    },
    /// Execute a question by ID
    #[command(after_help = "Examples:
  mbr-cli query run 123                # Execute question 123
  mbr-cli query run 123 --format json  # Output as JSON
  mbr-cli query run 123 --format csv   # Output as CSV
  mbr-cli query run 123 --param date=2024-01-01  # With parameters")]
    Run {
        /// Question ID to execute
        id: u32,

        /// Query parameters in key=value format (can be repeated)
        #[arg(long, action = clap::ArgAction::Append)]
        param: Vec<String>,

        /// Output format: table, json, or csv
        #[arg(short, long, default_value = "table")]
        format: String,

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

        /// Display only specified columns (comma-separated names)
        #[arg(long)]
        columns: Option<String>,

        /// Number of rows per page in interactive mode
        #[arg(long, default_value = "20")]
        page_size: usize,
    },
}

/// Query arguments for executing or listing questions.
///
/// Supports both:
/// - New subcommand style: `query list`, `query run <ID>`
/// - Legacy flag style: `query --list`, `query <ID>` (backward compatible)
#[derive(Args, Debug, Clone)]
#[command(
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true,
    after_help = "Examples:
  mbr-cli query list                   # List all questions (recommended)
  mbr-cli query run 123                # Execute question 123 (recommended)
  mbr-cli query --list                 # List all questions (legacy)
  mbr-cli query --list --search sales  # Search questions (legacy)
  mbr-cli query 123                    # Execute question 123 (legacy)
  mbr-cli query 123 --format json      # Output as JSON
  mbr-cli query 123 --format csv       # Output as CSV
  mbr-cli query 123 --param date=2024-01-01  # With parameters"
)]
pub struct QueryArgs {
    /// Subcommand (list or run)
    #[command(subcommand)]
    pub subcommand: Option<QueryCommand>,

    /// Question ID to execute (omit with --list to show questions)
    pub id: Option<u32>,

    /// List available questions instead of executing (deprecated: use `query list`)
    #[arg(long, short = 'l', help_heading = "Legacy Options", hide = true)]
    pub list: bool,

    /// Search term to filter questions (deprecated: use `query list --search`)
    #[arg(long, help_heading = "Legacy Options", hide = true)]
    pub search: Option<String>,

    /// Filter by collection ID (deprecated: use `query list --collection`)
    #[arg(long, help_heading = "Legacy Options", hide = true)]
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

impl QueryArgs {
    /// Normalize legacy flag-style args into the new QueryCommand subcommand form.
    /// Returns a resolved QueryCommand or an error message.
    pub fn resolve(&self) -> Result<ResolvedQuery, String> {
        // If a new-style subcommand was used, it takes priority
        if let Some(ref cmd) = self.subcommand {
            return Ok(match cmd {
                QueryCommand::List {
                    search,
                    collection,
                    limit,
                } => ResolvedQuery::List {
                    search: search.clone(),
                    collection: collection.clone(),
                    limit: *limit,
                },
                QueryCommand::Run {
                    id,
                    param,
                    format,
                    limit,
                    full,
                    no_fullscreen,
                    offset,
                    columns,
                    page_size,
                } => ResolvedQuery::Run {
                    id: *id,
                    param: param.clone(),
                    format: format.clone(),
                    limit: *limit,
                    full: *full,
                    no_fullscreen: *no_fullscreen,
                    offset: *offset,
                    columns: columns.clone(),
                    page_size: *page_size,
                },
            });
        }

        // Legacy flag-based mode
        if self.list {
            return Ok(ResolvedQuery::List {
                search: self.search.clone(),
                collection: self.collection.clone(),
                limit: self.limit,
            });
        }

        if let Some(id) = self.id {
            return Ok(ResolvedQuery::Run {
                id,
                param: self.param.clone(),
                format: self.format.clone(),
                limit: self.limit,
                full: self.full,
                no_fullscreen: self.no_fullscreen,
                offset: self.offset,
                columns: self.columns.clone(),
                page_size: self.page_size,
            });
        }

        Err("Please provide a question ID to execute, or use `query list` to show available questions".to_string())
    }
}

/// Resolved query operation after normalizing legacy and new-style args.
#[derive(Debug, Clone)]
pub enum ResolvedQuery {
    List {
        search: Option<String>,
        collection: Option<String>,
        limit: u32,
    },
    Run {
        id: u32,
        param: Vec<String>,
        format: String,
        limit: u32,
        full: bool,
        no_fullscreen: bool,
        offset: Option<usize>,
        columns: Option<String>,
        page_size: usize,
    },
}
