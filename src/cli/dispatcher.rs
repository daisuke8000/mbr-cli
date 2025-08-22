use crate::api::client::MetabaseClient;
use crate::cli::main_types::{AuthCommands, Commands, ConfigCommands, QuestionCommands};
use crate::core::auth::LoginInput;
use crate::core::services::auth_service::AuthService;
use crate::core::services::config_service::ConfigService;
use crate::display::{
    OperationStatus, ProgressSpinner, TableDisplay, TableHeaderInfoBuilder,
    display_status,
};
use crate::utils::data::OffsetManager;
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
        config: Config,
        mut credentials: Credentials,
        verbose: bool,
        api_key: Option<String>,
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
                    profile.metabase_url.clone(),
                    api_key.clone(),
                )?)
            } else {
                self.log_verbose("Creating client without API key (empty API key ignored)");
                Ok(MetabaseClient::new(profile.metabase_url.clone())?)
            }
        } else {
            self.log_verbose("Creating client without API key");
            Ok(MetabaseClient::new(profile.metabase_url.clone())?)
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
            Credentials::load(&self.credentials.profile_name).unwrap_or_else(|_| self.credentials.clone())
        };
        
        Ok(AuthService::new(credentials, client))
    }

    // Helper method to create ConfigService with current configuration
    fn create_config_service(&self) -> ConfigService {
        ConfigService::new(self.config.clone())
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
                self.log_verbose("Attempting auth login command using AuthService");

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

                let mut auth_service = self.create_auth_service(profile)?;
                match auth_service.authenticate(input.clone()).await {
                    Ok(_) => {
                        self.log_verbose("Authentication via AuthService succeeded");
                        println!("✅ Successfully logged in as {}", input.username);
                        println!("Connected to: {}", profile.metabase_url);
                        Ok(())
                    }
                    Err(e) => {
                        println!("❌ Login failed: {}", e);
                        Err(e)
                    }
                }
            }
            AuthCommands::Logout => {
                self.log_verbose("Attempting auth logout command using AuthService");
                
                let profile = self.get_current_profile()?;
                let mut auth_service = self.create_auth_service(profile)?;
                
                match auth_service.logout().await {
                    Ok(_) => {
                        println!(
                            "✅ Successfully logged out from profile: {}",
                            self.credentials.profile_name
                        );
                        Ok(())
                    }
                    Err(e) => {
                        println!("❌ Logout failed: {}", e);
                        Err(e)
                    }
                }
            }
            AuthCommands::Status => {
                self.log_verbose("Attempting auth status command using AuthService");

                let profile = self.get_current_profile()?;
                let auth_service = self.create_auth_service(profile)?;
                let auth_status = auth_service.get_auth_status();

                // Show authentication status
                println!("Authentication Status:");
                println!("=====================");

                match auth_status.auth_mode {
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

                        if auth_status.session_active {
                            if auth_service.is_authenticated() {
                                println!("Session: ✅ Active session found");
                                self.log_verbose("Valid session token found in keychain");
                            } else {
                                println!("Session: ❌ Session token invalid or expired");
                                self.log_verbose("Session token found but appears invalid");
                            }
                        } else {
                            println!(
                                "Session: ❌ No active session (use 'auth login' to authenticate)"
                            );
                            self.log_verbose("No session token found in keychain");
                        }
                    }
                }

                // Profile is determined by the current credentials (runtime profile)
                println!("\nActive Profile: {}", auth_status.profile_name);

                Ok(())
            }
        }
    }

    async fn handle_config_command(&self, commands: ConfigCommands) -> Result<(), AppError> {
        match commands {
            ConfigCommands::Show => {
                self.log_verbose("Attempting config show command using ConfigService");

                let config_service = self.create_config_service();
                let profiles = config_service.list_profiles();

                // Show the current configuration
                println!("Current Configuration:");
                println!("=====================");

                if let Some(default_profile) = &self.config.default_profile {
                    println!("Default Profile: {}", default_profile);
                } else {
                    println!("Default Profile: (not set)");
                }

                println!("\nProfiles:");
                if profiles.is_empty() {
                    println!("  No profiles configured");
                } else {
                    for (name, profile) in profiles {
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
                    "Attempting config set using ConfigService - profile: {}, field: {}, value: {}",
                    profile, field, value
                ));

                let mut config_service = self.create_config_service();

                // Validate URL if setting URL field
                if field.as_str() == "url" {
                    crate::utils::validation::validate_url(&value)?;
                }

                // Set the field using ConfigService
                config_service.set_profile_field(&profile, &field, &value)?;

                // Display success message - ConfigService handles field validation
                match field.as_str() {
                    "url" => {
                        println!("✅ Set profile '{}' URL to: {}", profile, value);
                    }
                    "email" => {
                        println!("✅ Set profile '{}' email to: {}", profile, value);
                    }
                    _ => {
                        // This should not happen as ConfigService validates fields first
                        return Err(AppError::Cli(CliError::InvalidArguments(format!(
                            "Invalid field: {}. Use 'url' or 'email'",
                            field
                        ))));
                    }
                }

                // Save the updated config
                config_service.save_config(None)?;

                println!("Configuration saved successfully.");
                Ok(())
            }
        }
    }

    async fn handle_question_command(&self, commands: QuestionCommands) -> Result<(), AppError> {
        match commands {
            QuestionCommands::Execute {
                id,
                param,
                format,
                limit,
                full,
                no_fullscreen,
                offset,
                columns,
                page_size,
            } => {
                self.log_verbose(&format!(
                    "Attempting question execute command - ID: {}, Params: {:?}, Format: {}, Limit: {:?}, Full: {}, Offset: {:?}, Page size: {}",
                    id, param, format, limit, full, offset, page_size
                ));

                // Get profile for API connection
                let profile = self.get_current_profile()?;

                // Create authenticated API client
                let client = self.create_authenticated_client(profile)?;

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

                // Show progress while executing question
                let mut spinner = ProgressSpinner::new(format!("Executing question {}...", id));
                spinner.start();

                // Execute the question
                let result = client.execute_question(id, parameters).await?;
                spinner.stop(Some("✅ Question execution completed"));

                let original_row_count = result.data.rows.len();
                let mut processed_result = result;

                // Apply offset if specified
                if let Some(offset_val) = offset {
                    if offset_val > 0 {
                        let offset_manager = OffsetManager::new(Some(offset_val))?;
                        processed_result = offset_manager.apply_offset(&processed_result)?;
                        self.log_verbose(&format!(
                            "Applied offset: {}, remaining rows: {}",
                            offset_val,
                            processed_result.data.rows.len()
                        ));
                    }
                }

                // Create table display
                let table_display = TableDisplay::new();

                // Always show comprehensive header (no longer optional)
                let display_start = offset.map(|o| o + 1).unwrap_or(1);
                // Calculate actual displayed rows (minimum of limit and remaining rows after offset)
                let actual_displayed_rows = if let Some(limit_val) = limit {
                    processed_result.data.rows.len().min(limit_val as usize)
                } else {
                    processed_result.data.rows.len()
                };
                // Calculate display_end by adding actual displayed rows to start position (same as start if zero rows)
                let display_end = if actual_displayed_rows > 0 {
                    display_start + actual_displayed_rows - 1
                } else {
                    display_start
                };

                let header_info = TableHeaderInfoBuilder::new()
                    .data_source("Question execution result".to_string())
                    .source_id(id)
                    .total_records(original_row_count)
                    .display_range(display_start, display_end)
                    .offset(offset.unwrap_or(0))
                    .build()?;

                print!(
                    "{}",
                    table_display.render_comprehensive_header(&header_info)
                );

                // Apply column filtering if specified
                let mut final_result = processed_result;
                if let Some(ref _column_filter) = columns {
                    // Column filtering feature not yet implemented
                    // Currently ignored - displays all columns
                }

                // Apply limit to the dataset if specified
                if let Some(limit_val) = limit {
                    final_result.data.rows = final_result
                        .data
                        .rows
                        .into_iter()
                        .take(limit_val as usize)
                        .collect();
                }

                // Output based on format
                match format.as_str() {
                    "json" => {
                        // JSON output - serialize and print
                        match serde_json::to_string_pretty(&final_result) {
                            Ok(json_output) => println!("{}", json_output),
                            Err(e) => {
                                eprintln!("Error serializing to JSON: {}", e);
                                return Err(AppError::Cli(CliError::InvalidArguments(format!(
                                    "Failed to serialize result to JSON: {}",
                                    e
                                ))));
                            }
                        }
                    }
                    "csv" => {
                        // CSV output
                        self.log_verbose("Rendering CSV output");

                        // Print CSV header
                        let headers: Vec<String> = final_result
                            .data
                            .cols
                            .iter()
                            .map(|col| col.display_name.clone())
                            .collect();
                        println!("{}", headers.join(","));

                        // Print CSV rows
                        for row in &final_result.data.rows {
                            let csv_row: Vec<String> = row
                                .iter()
                                .map(|cell| table_display.format_cell_value(cell))
                                .collect();
                            println!("{}", csv_row.join(","));
                        }
                    }
                    _ => {
                        // Table output (default)
                        if full {
                            // Full display without pagination
                            self.log_verbose("Using full display mode");
                            let rendered_table =
                                table_display.render_query_result(&final_result)?;
                            println!("{}", rendered_table);
                        } else if no_fullscreen {
                            // Simple pagination without interactive features
                            self.log_verbose("Using simple pagination mode");
                            let rendered_table = table_display
                                .render_query_result_with_limit(&final_result, Some(page_size))?;
                            println!("{}", rendered_table);
                        } else {
                            // Interactive mode - Full interactive display based on original implementation
                            self.log_verbose("Using full interactive mode with crossterm");

                            self.display_interactive_pagination(
                                &final_result,
                                page_size,
                                offset,
                                no_fullscreen,
                                id,
                                &format!("Question {}", id),
                            )
                            .await?;
                        }
                    }
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

                // Create authenticated API client
                let client = self.create_authenticated_client(profile)?;

                // Show progress while fetching questions
                let mut spinner = ProgressSpinner::new("Fetching questions...".to_string());
                spinner.start();

                // Call list_questions with optional parameters
                let questions = client
                    .list_questions(search.as_deref(), Some(limit), collection.as_deref())
                    .await?;

                spinner.stop(Some("✅ Questions fetched successfully"));

                // Display results using TableDisplay
                if questions.is_empty() {
                    display_status("Question search", OperationStatus::Warning);
                    println!("No questions found matching the criteria.");
                } else {
                    display_status(
                        &format!("Retrieved {} questions", questions.len()),
                        OperationStatus::Success,
                    );

                    // Interactive mode for question list
                    self.display_interactive_question_list(&questions, limit as usize)
                        .await?;
                }
                Ok(())
            }
        }
    }

    /// Display query results with interactive pagination using InteractiveDisplay
    async fn display_interactive_pagination(
        &self,
        result: &crate::api::models::QueryResult,
        page_size: usize,
        initial_offset: Option<usize>,
        no_fullscreen: bool,
        question_id: u32,
        question_name: &str,
    ) -> Result<(), AppError> {
        let display = crate::cli::interactive_display::InteractiveDisplay::new();
        display.display_query_result_pagination(
            result,
            page_size,
            initial_offset,
            no_fullscreen,
            question_id,
            question_name,
        ).await
    }

    /// Display question list with interactive pagination using InteractiveDisplay
    async fn display_interactive_question_list(
        &self,
        questions: &[crate::api::models::Question],
        page_size: usize,
    ) -> Result<(), AppError> {
        let display = crate::cli::interactive_display::InteractiveDisplay::new();
        display.display_question_list_pagination(questions, page_size).await
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
}
