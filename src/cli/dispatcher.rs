use crate::api::client::MetabaseClient;
use crate::cli::command_handlers::{AuthHandler, ConfigHandler, QuestionHandler};
use crate::cli::dashboard_handler::DashboardHandler;
use crate::cli::main_types::Commands;
use crate::core::services::auth_service::AuthService;
use crate::core::services::config_service::ConfigService;
use crate::error::{AppError, CliError};
use crate::storage::config::{Config, Profile};
use crate::storage::credentials::{AuthMode, Credentials};

pub struct Dispatcher {
    config: Config,
    credentials: Credentials,
    verbose: bool,
    api_key: Option<String>,
}

impl Dispatcher {
    // Static helper function for verbose logging (used before self exists)
    fn print_verbose(verbose: bool, msg: &str) {
        if verbose {
            println!("Verbose: {}", msg);
        }
    }

    // Instance method for verbose logging
    fn log_verbose(&self, msg: &str) {
        Self::print_verbose(self.verbose, msg);
    }

    pub fn new(
        mut config: Config,
        mut credentials: Credentials,
        verbose: bool,
        api_key: Option<String>,
        config_path: Option<std::path::PathBuf>,
    ) -> Self {
        // Session auto-restoration logic
        // Skip if an API key is set (an API key has priority)
        let has_valid_api_key = api_key.as_ref().is_some_and(|key| !key.is_empty());
        if matches!(credentials.get_auth_mode(), AuthMode::Session) && !has_valid_api_key {
            Self::print_verbose(verbose, "Checking for saved session token...");

            match Credentials::load(&credentials.profile_name) {
                Ok(loaded_creds) => {
                    credentials = loaded_creds;
                    Self::print_verbose(
                        verbose,
                        &format!(
                            "Session credentials loaded for profile: {}",
                            credentials.profile_name
                        ),
                    );
                }
                Err(_) => {
                    Self::print_verbose(
                        verbose,
                        &format!(
                            "No saved session token found for profile: {}",
                            credentials.profile_name
                        ),
                    );
                }
            }
        } else {
            Self::print_verbose(verbose, "API key is set, skipping session restoration");
        }

        // Create default profile if it doesn't exist
        if config.get_profile(&credentials.profile_name).is_none() {
            Self::print_verbose(
                verbose,
                &format!("Creating default profile: {}", credentials.profile_name),
            );

            use crate::storage::config::Profile;
            let default_profile = Profile {
                url: "http://localhost:3000".to_string(),
                email: None,
            };

            config.set_profile(credentials.profile_name.clone(), default_profile);

            // Save the updated config
            if let Err(err) = config.save(config_path) {
                Self::print_verbose(verbose, &format!("Warning: Failed to save config: {}", err));
            } else {
                Self::print_verbose(verbose, "Successfully saved config file");
            }
        }

        Self {
            config,
            credentials,
            verbose,
            api_key,
        }
    }

    // Helper method to get profile for the current credentials
    fn get_current_profile(&self) -> Result<&Profile, AppError> {
        self.config
            .get_profile(&self.credentials.profile_name)
            .ok_or_else(|| {
                AppError::Cli(CliError::AuthRequired {
                    message: format!("Profile '{}' not found", self.credentials.profile_name),
                    hint: "Use 'mbr-cli config set <profile> <field> <value>' to create a profile"
                        .to_string(),
                    available_profiles: self.config.profiles.keys().cloned().collect(),
                })
            })
    }

    // Helper method to create MetabaseClient with API key integration
    fn create_client(&self, profile: &Profile) -> Result<MetabaseClient, AppError> {
        if let Some(ref api_key) = self.api_key {
            if !api_key.is_empty() {
                self.log_verbose("Creating client with API key");
                Ok(MetabaseClient::with_api_key(
                    profile.url.clone(),
                    api_key.clone(),
                )?)
            } else {
                self.log_verbose("Creating client without API key (empty API key ignored)");
                Ok(MetabaseClient::new(profile.url.clone())?)
            }
        } else {
            self.log_verbose("Creating client without API key");
            Ok(MetabaseClient::new(profile.url.clone())?)
        }
    }

    // Helper method to create authenticated MetabaseClient with session restoration
    fn create_authenticated_client(&self, profile: &Profile) -> Result<MetabaseClient, AppError> {
        let mut client = self.create_client(profile)?;

        // For session-based authentication, restore session if available
        let has_valid_api_key = self.api_key.as_ref().is_some_and(|key| !key.is_empty());
        if matches!(self.credentials.get_auth_mode(), AuthMode::Session) && !has_valid_api_key {
            self.log_verbose("Attempting to restore session token");
            let credentials = Credentials::load(&self.credentials.profile_name)?;
            if let Some(token) = credentials.get_session_token() {
                self.log_verbose("Session token found, setting for client");
                client.set_session_token(token);
            } else {
                self.log_verbose("No session token found");
            }
        }

        Ok(client)
    }

    // Helper method to create AuthService with proper credentials and client
    fn create_auth_service(&self, profile: &Profile) -> Result<AuthService, AppError> {
        let client = self.create_authenticated_client(profile)?;
        let credentials = if self.api_key.is_some() {
            // Use credentials with API key preference
            self.credentials.clone()
        } else {
            // Load latest credentials from storage for session mode
            Credentials::load(&self.credentials.profile_name)
                .unwrap_or_else(|_| self.credentials.clone())
        };

        Ok(AuthService::new(credentials, client))
    }

    // Helper method to create ConfigService with current configuration
    fn create_config_service(&self) -> ConfigService {
        ConfigService::new(self.config.clone())
    }

    pub async fn dispatch(&self, command: Commands) -> Result<(), AppError> {
        match command {
            Commands::Auth { command } => {
                let handler = AuthHandler::new();
                let profile = self.get_current_profile()?;
                let mut auth_service = self.create_auth_service(profile)?;
                handler
                    .handle(command, &mut auth_service, profile, self.verbose)
                    .await
            }
            Commands::Config { command } => {
                let handler = ConfigHandler::new();
                let mut config_service = self.create_config_service();
                handler
                    .handle(command, &mut config_service, self.verbose)
                    .await
            }
            Commands::Question { command } => {
                let handler = QuestionHandler::new();
                let profile = self.get_current_profile()?;
                let client = self.create_authenticated_client(profile)?;
                handler.handle(command, client, self.verbose).await
            }
            Commands::Dashboard { command } => {
                let handler = DashboardHandler::new();
                let profile = self.get_current_profile()?;
                let client = self.create_authenticated_client(profile)?;
                handler
                    .handle_dashboard_commands(&client, &command, self.verbose)
                    .await
            }
        }
    }
}
