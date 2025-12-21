use crate::cli::command_handlers::{
    CollectionHandler, ConfigHandler, DatabaseHandler, QueryHandler,
};
use crate::cli::main_types::Commands;
use mbr_core::api::client::MetabaseClient;
use mbr_core::core::services::config_service::ConfigService;
use mbr_core::error::{AppError, CliError};
use mbr_core::storage::config::Config;
use mbr_core::storage::credentials::get_api_key;
use mbr_core::utils::logging::print_verbose;

pub struct Dispatcher {
    config: Config,
    verbose: bool,
    api_key: Option<String>,
}

impl Dispatcher {
    fn log_verbose(&self, msg: &str) {
        print_verbose(self.verbose, msg);
    }

    pub fn new(config: Config, verbose: bool, api_key: Option<String>) -> Self {
        Self {
            config,
            verbose,
            api_key,
        }
    }

    // Get effective API key (CLI arg > env var)
    fn get_effective_api_key(&self) -> Option<String> {
        // CLI argument takes priority
        if let Some(ref key) = self.api_key {
            if !key.is_empty() {
                return Some(key.clone());
            }
        }
        // Fall back to environment variable
        get_api_key()
    }

    // Get URL from config or environment
    fn get_url(&self) -> Result<String, AppError> {
        self.config.get_url().ok_or_else(|| {
            AppError::Cli(CliError::InvalidArguments(
                "Metabase URL is not configured. Use 'mbr-cli config set --url <url>' or set MBR_URL environment variable".to_string(),
            ))
        })
    }

    // Helper method to create MetabaseClient with API key
    fn create_client(&self) -> Result<MetabaseClient, AppError> {
        let url = self.get_url()?;
        if let Some(api_key) = self.get_effective_api_key() {
            self.log_verbose("Creating client with API key");
            Ok(MetabaseClient::with_api_key(url, api_key)?)
        } else {
            self.log_verbose("Creating client without API key");
            Ok(MetabaseClient::new(url)?)
        }
    }

    // Helper method to create ConfigService with current configuration
    fn create_config_service(&self) -> ConfigService {
        ConfigService::new(self.config.clone())
    }

    pub async fn dispatch(&self, command: Commands) -> Result<(), AppError> {
        match command {
            Commands::Config { command } => {
                let handler = ConfigHandler::new();
                let mut config_service = self.create_config_service();
                // For config commands, we may not have a URL yet
                let client = match self.create_client() {
                    Ok(c) => c,
                    Err(_) => {
                        // Create a dummy client for config show/set (URL not needed)
                        MetabaseClient::new("http://localhost:3000".to_string())?
                    }
                };
                handler
                    .handle(command, &mut config_service, client, self.verbose)
                    .await
            }
            Commands::Query(args) => {
                let handler = QueryHandler::new();
                let client = self.create_client()?;
                handler.handle(args, client, self.verbose).await
            }
            Commands::Collection { command } => {
                let handler = CollectionHandler::new();
                let client = self.create_client()?;
                handler.handle(command, client, self.verbose).await
            }
            Commands::Database { command } => {
                let handler = DatabaseHandler::new();
                let client = self.create_client()?;
                handler.handle(command, client, self.verbose).await
            }
        }
    }
}
