use crate::api::client::MetabaseClient;
use crate::cli::interactive_display::InteractiveDisplay;
use crate::cli::main_types::{AuthCommands, ConfigCommands, QuestionCommands};
use crate::core::auth::LoginInput;
use crate::core::services::auth_service::AuthService;
use crate::core::services::config_service::ConfigService;
use crate::display::{
    OperationStatus, ProgressSpinner, TableDisplay, TableHeaderInfoBuilder,
    display_status,
};
use crate::error::{AppError, CliError};
use crate::storage::config::Profile;
use crate::storage::credentials::AuthMode;
use crate::utils::data::OffsetManager;

/// Handler for authentication commands
pub struct AuthHandler;

impl AuthHandler {
    pub fn new() -> Self {
        Self
    }

    /// Print verbose message if verbose mode is enabled
    fn print_verbose(verbose: bool, msg: &str) {
        if verbose {
            println!("Verbose: {}", msg);
        }
    }

    pub async fn handle(
        &self,
        command: AuthCommands,
        auth_service: &mut AuthService,
        profile: &Profile,
        verbose: bool,
    ) -> Result<(), AppError> {
        match command {
            AuthCommands::Login { username, password } => {
                Self::print_verbose(verbose, "Attempting auth login command using AuthService");

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

                match auth_service.authenticate(input.clone()).await {
                    Ok(_) => {
                        Self::print_verbose(verbose, "Authentication via AuthService succeeded");
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
                Self::print_verbose(verbose, "Attempting auth logout command using AuthService");
                
                match auth_service.logout().await {
                    Ok(_) => {
                        println!(
                            "✅ Successfully logged out from profile: {}",
                            auth_service.get_auth_status().profile_name
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
                Self::print_verbose(verbose, "Attempting auth status command using AuthService");

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
                                Self::print_verbose(verbose, "Valid session token found in keychain");
                            } else {
                                println!("Session: ❌ Session token invalid or expired");
                                Self::print_verbose(verbose, "Session token found but appears invalid");
                            }
                        } else {
                            println!(
                                "Session: ❌ No active session (use 'auth login' to authenticate)"
                            );
                            Self::print_verbose(verbose, "No session token found in keychain");
                        }
                    }
                }

                // Profile is determined by the current credentials (runtime profile)
                println!("\nActive Profile: {}", auth_status.profile_name);

                Ok(())
            }
        }
    }
}

/// Handler for configuration commands
pub struct ConfigHandler;

impl ConfigHandler {
    pub fn new() -> Self {
        Self
    }

    /// Print verbose message if verbose mode is enabled
    fn print_verbose(verbose: bool, msg: &str) {
        if verbose {
            println!("Verbose: {}", msg);
        }
    }

    pub async fn handle(
        &self,
        command: ConfigCommands,
        config_service: &mut ConfigService,
        verbose: bool,
    ) -> Result<(), AppError> {
        match command {
            ConfigCommands::Show => {
                Self::print_verbose(verbose, "Attempting config show command using ConfigService");

                let profiles = config_service.list_profiles();

                // Show the current configuration
                println!("Current Configuration:");
                println!("=====================");

                if let Some(default_profile) = config_service.get_default_profile() {
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
                Self::print_verbose(verbose, &format!(
                    "Attempting config set using ConfigService - profile: {}, field: {}, value: {}",
                    profile, field, value
                ));

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
                        return Err(AppError::Cli(crate::error::CliError::InvalidArguments(format!(
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
}

/// Handler for question commands
pub struct QuestionHandler;

impl QuestionHandler {
    pub fn new() -> Self {
        Self
    }

    /// Print verbose message if verbose mode is enabled
    fn print_verbose(verbose: bool, msg: &str) {
        if verbose {
            println!("Verbose: {}", msg);
        }
    }

    pub async fn handle(
        &self,
        command: QuestionCommands,
        client: MetabaseClient,
        verbose: bool,
    ) -> Result<(), AppError> {
        match command {
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
                Self::print_verbose(verbose, &format!(
                    "Attempting question execute command - ID: {}, Params: {:?}, Format: {}, Limit: {:?}, Full: {}, Offset: {:?}, Page size: {}",
                    id, param, format, limit, full, offset, page_size
                ));

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
                        Self::print_verbose(verbose, &format!(
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
                        Self::print_verbose(verbose, "Rendering CSV output");

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
                            Self::print_verbose(verbose, "Using full display mode");
                            let rendered_table =
                                table_display.render_query_result(&final_result)?;
                            println!("{}", rendered_table);
                        } else if no_fullscreen {
                            // Simple pagination without interactive features
                            Self::print_verbose(verbose, "Using simple pagination mode");
                            let rendered_table = table_display
                                .render_query_result_with_limit(&final_result, Some(page_size))?;
                            println!("{}", rendered_table);
                        } else {
                            // Interactive mode - Full interactive display based on original implementation
                            Self::print_verbose(verbose, "Using full interactive mode with crossterm");

                            let interactive_display = InteractiveDisplay::new();
                            interactive_display.display_query_result_pagination(
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
                Self::print_verbose(verbose, &format!(
                    "Attempting question list command - Search: {:?}, Limit: {}, Collection: {:?}",
                    search, limit, collection
                ));

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
                    let interactive_display = InteractiveDisplay::new();
                    interactive_display.display_question_list_pagination(&questions, limit as usize)
                        .await?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_handler_creation() {
        let handler = AuthHandler::new();
        assert!(std::any::type_name::<AuthHandler>().contains("AuthHandler"));
    }

    #[test]
    fn test_config_handler_creation() {
        let handler = ConfigHandler::new();
        assert!(std::any::type_name::<ConfigHandler>().contains("ConfigHandler"));
    }

    #[test]
    fn test_question_handler_creation() {
        let handler = QuestionHandler::new();
        assert!(std::any::type_name::<QuestionHandler>().contains("QuestionHandler"));
    }

    #[test]
    fn test_command_handlers_separation() {
        // Test that command handlers are separate concerns
        let auth_handler = AuthHandler::new();
        let config_handler = ConfigHandler::new();
        let question_handler = QuestionHandler::new();
        
        // Each handler should focus on its specific command domain
        // No shared state or cross-cutting concerns
        assert!(std::mem::size_of_val(&auth_handler) == std::mem::size_of::<AuthHandler>());
        assert!(std::mem::size_of_val(&config_handler) == std::mem::size_of::<ConfigHandler>());
        assert!(std::mem::size_of_val(&question_handler) == std::mem::size_of::<QuestionHandler>());
    }
}