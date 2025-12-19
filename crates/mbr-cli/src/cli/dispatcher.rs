use crate::cli::command_handlers::{ConfigHandler, QueryHandler};
use crate::cli::main_types::Commands;
use mbr_core::api::client::MetabaseClient;
use mbr_core::core::services::config_service::ConfigService;
use mbr_core::error::{AppError, CliError};
use mbr_core::storage::config::{Config, Profile};
use mbr_core::storage::credentials::get_api_key;
use mbr_core::utils::logging::print_verbose;

pub struct Dispatcher {
    config: Config,
    profile_name: String,
    verbose: bool,
    api_key: Option<String>,
}

impl Dispatcher {
    fn log_verbose(&self, msg: &str) {
        print_verbose(self.verbose, msg);
    }

    pub fn new(
        mut config: Config,
        profile_name: String,
        verbose: bool,
        api_key: Option<String>,
        config_path: Option<std::path::PathBuf>,
    ) -> Self {
        // Create default profile if not exists
        if config.get_profile(&profile_name).is_none() {
            print_verbose(
                verbose,
                &format!("Creating default profile: {}", profile_name),
            );

            let default_profile = Profile {
                url: "http://localhost:3000".to_string(),
                email: None,
            };

            config.set_profile(profile_name.clone(), default_profile);

            if let Err(err) = config.save(config_path) {
                print_verbose(verbose, &format!("Warning: Failed to save config: {}", err));
            } else {
                print_verbose(verbose, "Successfully saved config file");
            }
        }

        Self {
            config,
            profile_name,
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

    // Helper method to get profile for the current credentials
    fn get_current_profile(&self) -> Result<&Profile, AppError> {
        self.config.get_profile(&self.profile_name).ok_or_else(|| {
            AppError::Cli(CliError::AuthRequired {
                message: format!("Profile '{}' not found", self.profile_name),
                hint: "Use 'mbr-cli config set --profile <name> --url <url>' to create a profile"
                    .to_string(),
                available_profiles: self.config.profiles.keys().cloned().collect(),
            })
        })
    }

    // Helper method to create MetabaseClient with API key
    fn create_client(&self, profile: &Profile) -> Result<MetabaseClient, AppError> {
        if let Some(api_key) = self.get_effective_api_key() {
            self.log_verbose("Creating client with API key");
            Ok(MetabaseClient::with_api_key(profile.url.clone(), api_key)?)
        } else {
            self.log_verbose("Creating client without API key");
            Ok(MetabaseClient::new(profile.url.clone())?)
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
                let profile = self.get_current_profile()?;
                let client = self.create_client(profile)?;
                handler
                    .handle(command, &mut config_service, client, self.verbose)
                    .await
            }
            Commands::Query(args) => {
                let handler = QueryHandler::new();
                let profile = self.get_current_profile()?;
                let client = self.create_client(profile)?;
                handler.handle(args, client, self.verbose).await
            }
        }
    }
}
