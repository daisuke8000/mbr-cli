use crate::api::client::MetabaseClient;
use crate::cli::main_types::{AuthCommands, Commands, ConfigCommands, QuestionCommands};
use crate::core::auth::LoginInput;
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

    pub fn new(config: Config, mut credentials: Credentials, verbose: bool, api_key: Option<String>) -> Self {
        // Session auto-restoration logic
        // Skip if an API key is set (an API key has priority)
        if matches!(credentials.get_auth_mode(), AuthMode::Session) {
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

        Self {
            config,
            credentials,
            verbose,
            api_key,
        }
    }

    // Helper method to get profile for the current credentials
    fn get_current_profile(&self) -> Result<&Profile, AppError> {
        self.config.get_profile(&self.credentials.profile_name)
            .ok_or_else(|| AppError::Cli(CliError::AuthRequired {
                message: format!("Profile '{}' not found", self.credentials.profile_name),
                hint: "Use 'mbr-cli config set <profile> <field> <value>' to create a profile".to_string(),
                available_profiles: self.config.profiles.keys().cloned().collect(),
            }))
    }

    // Helper method to create MetabaseClient with API key integration
    fn create_client(&self, profile: &Profile) -> Result<MetabaseClient, AppError> {
        if let Some(ref api_key) = self.api_key {
            self.log_verbose("Creating client with API key");
            Ok(MetabaseClient::with_api_key(profile.metabase_url.clone(), api_key.clone())?)
        } else {
            self.log_verbose("Creating client without API key");
            Ok(MetabaseClient::new(profile.metabase_url.clone())?)
        }
    }

    pub async fn dispatch(&self, command: Commands) -> Result<(), AppError> {
        match command {
            Commands::Auth { command } => self.handle_auth_command(command).await,
            Commands::Config { command } => self.handle_config_command(command).await,
            Commands::Question { command } => self.handle_question_command(command).await,
        }
    }

    async fn handle_auth_command(&self, commands: AuthCommands) -> Result<(), AppError> {
        match commands {
            AuthCommands::Login { username, password } => {
                self.log_verbose("Attempting auth login command");

                // Get a profile to check for stored email
                let profile = self.get_current_profile()?;

                // Use CLI arguments if provided, otherwise collect interactively
                let input = if let (Some(user), Some(pass)) = (&username, &password) {
                    // Non-interactive mode: use provided credentials
                    LoginInput {
                        username: user.clone(),
                        password: pass.clone(),
                    }
                } else {
                    // Interactive mode: collect credentials via prompts
                    // Use profile email as default username if available and no username provided
                    let default_username = username.or_else(|| profile.email.clone());
                    LoginInput::collect(default_username.as_deref())?
                };
                input.validate()?;

                let mut client = self.create_client(profile)?;
                match client.login(&input.username, &input.password).await {
                    Ok(_) => {
                        if let Some(token) = client.get_session_token() {
                            Credentials::save_session_for_profile(
                                &self.credentials.profile_name,
                                &token,
                            )?;
                        }

                        println!("✅ Successfully logged in as {}", input.username);
                        println!("Connected to: {}", profile.metabase_url);
                        Ok(())
                    }
                    Err(e) => {
                        println!("❌ Login failed: {}", e);
                        Err(AppError::Api(e))
                    }
                }
            }
            AuthCommands::Logout => {
                self.log_verbose("Attempting auth logout command");
                Credentials::clear_session_for_profile(&self.credentials.profile_name)?;
                println!(
                    "✅ Successfully logged out from profile: {}",
                    self.credentials.profile_name
                );
                Ok(())
            }
            AuthCommands::Status => {
                self.log_verbose("Attempting auth status command");

                // Show authentication status
                println!("Authentication Status:");
                println!("=====================");

                let auth_mode = self.credentials.get_auth_mode();
                match auth_mode {
                    AuthMode::APIKey => {
                        println!("Authentication Mode: API Key");
                        // API key is set via environment variable
                        if let Ok(key) = std::env::var("MBR_API_KEY") {
                            // Mask the API key for security
                            let masked = if key.len() > 8 {
                                format!("{}...{}", &key[..4], &key[key.len() - 4..])
                            } else {
                                "*****".to_string()
                            };
                            println!("API Key: {}", masked);
                        } else {
                            println!("API Key: (not set)");
                        }
                    }
                    AuthMode::Session => {
                        println!("Authentication Mode: Session");
                        // Session token would be stored in the keyring
                        println!("Session: Check using 'auth login' to authenticate");
                    }
                }

                // Profile is determined by the current config
                if let Some(profile) = &self.config.default_profile {
                    println!("\nActive Profile: {}", profile);
                } else {
                    println!("\nActive Profile: (default)");
                }

                Ok(())
            }
        }
    }

    async fn handle_config_command(&self, commands: ConfigCommands) -> Result<(), AppError> {
        match commands {
            ConfigCommands::Show => {
                self.log_verbose("Attempting config show command");

                // Show the current configuration
                println!("Current Configuration:");
                println!("=====================");

                if let Some(default_profile) = &self.config.default_profile {
                    println!("Default Profile: {}", default_profile);
                } else {
                    println!("Default Profile: (not set)");
                }

                println!("\nProfiles:");
                if self.config.profiles.is_empty() {
                    println!("  No profiles configured");
                } else {
                    for (name, profile) in &self.config.profiles {
                        println!("  [{}]", name);
                        println!("    URL: {}", profile.metabase_url);
                        if let Some(email) = &profile.email {
                            println!("    Email: {}", email);
                        }
                    }
                }

                Ok(())
            }
            ConfigCommands::Set {
                profile,
                field,
                value,
            } => {
                self.log_verbose(&format!(
                    "Attempting config set - profile: {}, field: {}, value: {}",
                    profile, field, value
                ));

                // Clone config for modification
                let mut config = self.config.clone();

                // Get or create a profile
                let prof = config
                    .profiles
                    .entry(profile.to_string())
                    .or_insert_with(|| Profile {
                        metabase_url: String::new(),
                        email: None,
                    });

                // Set the field
                match field.as_str() {
                    "url" => {
                        // Validate URL
                        if !value.starts_with("http://") && !value.starts_with("https://") {
                            return Err(AppError::Cli(CliError::InvalidArguments(format!(
                                "Invalid URL: {}. Must start with http:// or https://",
                                value
                            ))));
                        }
                        prof.metabase_url = value.to_string();
                        println!("✅ Set profile '{}' URL to: {}", profile, value);
                    }
                    "email" => {
                        prof.email = Some(value.to_string());
                        println!("✅ Set profile '{}' email to: {}", profile, value);
                    }
                    _ => {
                        return Err(AppError::Cli(CliError::InvalidArguments(format!(
                            "Invalid field: {}. Use 'url' or 'email'",
                            field
                        ))));
                    }
                }

                // If this is the first profile or named "default", set it as default
                if config.profiles.len() == 1 || profile == "default" {
                    config.default_profile = Some(profile.to_string());
                }

                // Save the updated config to a file
                config.save(None).map_err(|e| {
                    AppError::Cli(CliError::InvalidArguments(format!(
                        "Failed to save config: {}",
                        e
                    )))
                })?;

                println!("Configuration saved successfully.");
                Ok(())
            }
        }
    }

    async fn handle_question_command(&self, commands: QuestionCommands) -> Result<(), AppError> {
        match commands {
            QuestionCommands::Execute { id, param, limit } => {
                self.log_verbose(&format!(
                    "Attempting question execute command - ID: {}, Params: {:?}, Limit: {:?}",
                    id, param, limit
                ));

                // Get profile for API connection
                let profile = self.get_current_profile()?;

                // Create API client
                let client = self.create_client(profile)?;

                // Convert parameters from Vec<String> to HashMap<String, String>
                let parameters = if param.is_empty() {
                    None
                } else {
                    let mut param_map = std::collections::HashMap::new();
                    for param_str in param {
                        // Parse parameter string (expected format: key=value)
                        if let Some((key, value)) = param_str.split_once('=') {
                            param_map.insert(key.to_string(), value.to_string());
                        } else {
                            println!(
                                "Warning: Invalid parameter format '{}'. Expected 'key=value'",
                                param_str
                            );
                        }
                    }
                    if param_map.is_empty() {
                        None
                    } else {
                        Some(param_map)
                    }
                };

                // Execute the question
                let result = client.execute_question(id, parameters).await?;

                // Display results with optional limit
                println!("Query executed successfully for question ID: {}", id);
                println!("Columns: {}", result.data.cols.len());
                println!("Rows: {}", result.data.rows.len());

                // Display column headers
                if !result.data.cols.is_empty() {
                    print!("| ");
                    for col in &result.data.cols {
                        print!("{:15} | ", col.display_name);
                    }
                    println!();

                    // Display separator
                    print!("|");
                    for _ in &result.data.cols {
                        print!("-----------------+");
                    }
                    println!();
                }

                // Display data rows with optional limit
                let rows_to_show = if let Some(limit_value) = limit {
                    std::cmp::min(limit_value as usize, result.data.rows.len())
                } else {
                    result.data.rows.len()
                };

                for (i, row) in result.data.rows.iter().enumerate() {
                    if i >= rows_to_show {
                        break;
                    }
                    print!("| ");
                    for value in row {
                        let display_value = match value {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            serde_json::Value::Null => "NULL".to_string(),
                            _ => value.to_string(),
                        };
                        print!(
                            "{:15} | ",
                            display_value.chars().take(15).collect::<String>()
                        );
                    }
                    println!();
                }

                if rows_to_show < result.data.rows.len() {
                    println!(
                        "... and {} more rows",
                        result.data.rows.len() - rows_to_show
                    );
                }

                Ok(())
            }
            QuestionCommands::List {
                search,
                limit,
                collection,
            } => {
                self.log_verbose(&format!(
                    "Attempting question list command - Search: {:?}, Limit: {}, Collection: {:?}",
                    search, limit, collection
                ));
                // Get profile for API connection
                let profile = self.get_current_profile()?;

                // Create API client
                let client = self.create_client(profile)?;

                // Call list_questions with optional parameters
                let questions = client
                    .list_questions(search.as_deref(), Some(limit), collection.as_deref())
                    .await?;

                // Display results
                if questions.is_empty() {
                    println!("No questions found.");
                } else {
                    println!("Found {} question(s):", questions.len());
                    for question in questions {
                        println!(
                            "  [{}] {} (Collection: {})",
                            question.id,
                            question.name,
                            question
                                .collection_id
                                .map_or("None".to_string(), |id| id.to_string())
                        );
                    }
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::config::Profile;
    use std::collections::HashMap;

    fn create_test_dispatcher(verbose: bool) -> Dispatcher {
        let config = Config {
            default_profile: Some("test".to_string()),
            profiles: {
                let mut profiles = HashMap::new();
                profiles.insert(
                    "test".to_string(),
                    Profile {
                        metabase_url: "http://example.test".to_string(),
                        email: None,
                    },
                );
                profiles
            },
        };
        let creds = Credentials::new("test".to_string());
        Dispatcher::new(config, creds, verbose, None)
    }

    #[tokio::test]
    async fn test_dispatcher_creation() {
        let d = create_test_dispatcher(true);
        assert!(d.verbose);
    }

    // Note: auth login requires interactive input, so we can't easily test the full flow
    // TODO: Add integration tests with mock stdin for auth login

    #[tokio::test]
    async fn test_auth_logout_implemented() {
        let d = create_test_dispatcher(true);
        let result = d.handle_auth_command(AuthCommands::Logout).await;
        // In a test environment, this should succeed (uses mock credentials)
        assert!(
            result.is_ok(),
            "Auth logout should succeed in test environment"
        );
    }

    #[tokio::test]
    async fn test_auth_status_implemented() {
        let d = create_test_dispatcher(true);
        let result = d.handle_auth_command(AuthCommands::Status).await;
        // auth status should now succeed
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_config_show_implemented() {
        let d = create_test_dispatcher(true);
        let result = d.handle_config_command(ConfigCommands::Show).await;
        // config show should now succeed
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_config_set_url() {
        let d = create_test_dispatcher(true);
        let result = d
            .handle_config_command(ConfigCommands::Set {
                profile: "test".to_string(),
                field: "url".to_string(),
                value: "http://localhost:3000".to_string(),
            })
            .await;
        // Config set should now succeed
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_question_list_implemented() {
        let d = create_test_dispatcher(true);
        let result = d
            .handle_question_command(QuestionCommands::List {
                search: None,
                limit: 10,
                collection: None,
            })
            .await;

        // Should get an API error (network error due to test URL), not NotImplemented
        assert!(result.is_err());
        match result {
            Err(AppError::Api(_)) => {
                // Expected: API error due to invalid test URL or network issues
            }
            Err(other) => {
                panic!("Expected API error, got: {:?}", other);
            }
            Ok(_) => {
                panic!("Expected error due to test environment");
            }
        }
    }

    #[tokio::test]
    async fn test_question_execute_implemented() {
        let d = create_test_dispatcher(true);
        let result = d
            .handle_question_command(QuestionCommands::Execute {
                id: 1,
                param: vec![
                    "sample_param=sample_value".to_string(),
                    "another_param=another_value".to_string(),
                ],
                limit: Some(20),
            })
            .await;

        // Should get an API error (network error due to test URL), not NotImplemented
        assert!(result.is_err());
        match result {
            Err(AppError::Api(_)) => {
                // Expected: API error due to invalid test URL or network issues
            }
            Err(other) => {
                panic!("Expected API error, got: {:?}", other);
            }
            Ok(_) => {
                panic!("Expected error due to test environment");
            }
        }
    }

    #[tokio::test]
    async fn test_dispatch_cmd() {
        let d = create_test_dispatcher(true);

        // Auth login will fail because it requires interactive input
        // In a real test environment, we'd need to mock stdin
        // For now, we skip testing the login command here

        // Config show should now succeed
        let result = d
            .dispatch(Commands::Config {
                command: ConfigCommands::Show,
            })
            .await;
        assert!(result.is_ok());

        // Auth status should now succeed
        let result = d
            .dispatch(Commands::Auth {
                command: AuthCommands::Status,
            })
            .await;
        assert!(result.is_ok());

        // Auth logout should succeed
        let result = d
            .dispatch(Commands::Auth {
                command: AuthCommands::Logout,
            })
            .await;
        assert!(result.is_ok());

        // Question list should still fail (not implemented)
        let result = d
            .dispatch(Commands::Question {
                command: QuestionCommands::List {
                    search: None,
                    limit: 10,
                    collection: None,
                },
            })
            .await;
        assert!(result.is_err());
    }
}
