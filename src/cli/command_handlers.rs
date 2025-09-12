use crate::api::client::MetabaseClient;
use crate::cli::interactive_display::InteractiveDisplay;
use crate::cli::main_types::{AuthCommands, ConfigCommands, QuestionCommands};
use crate::core::auth::LoginInput;
use crate::core::services::auth_service::AuthService;
use crate::core::services::config_service::ConfigService;
use crate::display::{
    OperationStatus, ProgressSpinner, TableDisplay, TableHeaderInfoBuilder, display_status,
};
use crate::error::{AppError, CliError};
use crate::storage::config::Profile;
use crate::storage::credentials::AuthMode;
use crate::utils::data::OffsetManager;
use crate::utils::logging::print_verbose;

#[derive(Default)]
pub struct AuthHandler;

impl AuthHandler {
    pub fn new() -> Self {
        Self
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
                print_verbose(verbose, "Attempting auth login command using AuthService");

                let default_username = username.clone().or_else(|| profile.email.clone());
                let input =
                    LoginInput::from_args_or_env(username, password, default_username.as_deref())?;

                match auth_service.authenticate(input.clone()).await {
                    Ok(_) => {
                        print_verbose(verbose, "Authentication via AuthService succeeded");
                        println!("✅ Successfully logged in as {}", input.username);
                        println!("Connected to: {}", profile.url);
                        Ok(())
                    }
                    Err(e) => {
                        println!("❌ Login failed: {}", e);
                        Err(e)
                    }
                }
            }
            AuthCommands::Logout => {
                print_verbose(verbose, "Attempting auth logout command using AuthService");

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
                print_verbose(verbose, "Attempting auth status command using AuthService");

                let auth_status = auth_service.get_auth_status();

                // Show authentication status
                println!("Authentication Status:");
                println!("=====================");

                match auth_status.auth_mode {
                    AuthMode::APIKey => {
                        println!("Authentication Mode: API Key");
                        if let Ok(key) = std::env::var("MBR_API_KEY") {
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
                                print_verbose(verbose, "Valid session token found in keychain");
                            } else {
                                println!("Session: ❌ Session token invalid or expired");
                                print_verbose(verbose, "Session token found but appears invalid");
                            }
                        } else {
                            println!(
                                "Session: ❌ No active session (use 'auth login' to authenticate)"
                            );
                            print_verbose(verbose, "No session token found in keychain");
                        }
                    }
                }

                println!("\nActive Profile: {}", auth_status.profile_name);

                Ok(())
            }
        }
    }
}

#[derive(Default)]
pub struct ConfigHandler;

impl ConfigHandler {
    pub fn new() -> Self {
        Self
    }

    pub async fn handle(
        &self,
        command: ConfigCommands,
        config_service: &mut ConfigService,
        verbose: bool,
    ) -> Result<(), AppError> {
        match command {
            ConfigCommands::Show => {
                print_verbose(
                    verbose,
                    "Attempting config show command using ConfigService",
                );

                let profiles = config_service.list_profiles();

                // Show the current configuration
                println!("Current Configuration:");
                println!("=====================");

                println!("Default Profile: {}", config_service.get_default_profile());

                println!("\nProfiles:");
                if profiles.is_empty() {
                    println!("  No profiles configured");
                } else {
                    for (name, profile) in profiles {
                        println!("  [{}]", name);
                        println!("    URL: {}", profile.url);
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
                url,
                email,
            } => {
                print_verbose(
                    verbose,
                    &format!(
                        "Attempting config set using ConfigService - profile: {}, field: {:?}, value: {:?}, url: {:?}, email: {:?}",
                        profile, field, value, url, email
                    ),
                );

                // Handle different input modes
                if let (Some(field), Some(value)) = (field, value) {
                    // Legacy mode: explicit field/value arguments
                    println!("Using explicit field/value arguments");

                    // Validate URL if setting URL field
                    if field.as_str() == "url" {
                        crate::utils::validation::validate_url(&value)?;
                    }

                    // Set the field using ConfigService
                    config_service.set_profile_field(&profile, &field, &value)?;

                    // Display success message
                    match field.as_str() {
                        "url" => println!("✅ Set profile '{}' URL to: {}", profile, value),
                        "email" => println!("✅ Set profile '{}' email to: {}", profile, value),
                        _ => {
                            return Err(AppError::Cli(crate::error::CliError::InvalidArguments(
                                format!("Invalid field: {}. Use 'url' or 'email'", field),
                            )));
                        }
                    }
                } else {
                    // Environment variable mode: use --url and --email flags (with env var fallback)
                    let mut updated_fields = Vec::new();

                    // Handle URL setting
                    if let Some(url_value) = url {
                        println!("Using URL from --url argument or MBR_URL environment variable");
                        crate::utils::validation::validate_url(&url_value)?;
                        config_service.set_profile_field(&profile, "url", &url_value)?;
                        updated_fields.push(format!("URL to: {}", url_value));
                    }

                    // Handle email setting
                    if let Some(email_value) = email {
                        println!(
                            "Using email from --email argument or MBR_USERNAME environment variable"
                        );
                        config_service.set_profile_field(&profile, "email", &email_value)?;
                        updated_fields.push(format!("email to: {}", email_value));
                    }

                    if updated_fields.is_empty() {
                        return Err(AppError::Cli(crate::error::CliError::InvalidArguments(
                            "No configuration values provided. Use --field/--value, --url, --email, or set MBR_URL/MBR_USERNAME environment variables".to_string(),
                        )));
                    }

                    println!("✅ Set profile '{}' {}", profile, updated_fields.join(", "));
                }

                config_service.save_config(None)?;

                println!("Configuration saved successfully.");
                Ok(())
            }
        }
    }
}

#[derive(Default)]
pub struct QuestionHandler;

impl QuestionHandler {
    pub fn new() -> Self {
        Self
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
                print_verbose(
                    verbose,
                    &format!(
                        "Attempting question execute command - ID: {}, Params: {:?}, Format: {}, Limit: {:?}, Full: {}, Offset: {:?}, Page size: {}",
                        id, param, format, limit, full, offset, page_size
                    ),
                );

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
                        print_verbose(
                            verbose,
                            &format!(
                                "Applied offset: {}, remaining rows: {}",
                                offset_val,
                                processed_result.data.rows.len()
                            ),
                        );
                    }
                }

                // Create table display
                let table_display = TableDisplay::new();

                let display_start = offset.map(|o| o + 1).unwrap_or(1);
                let actual_displayed_rows = if let Some(limit_val) = limit {
                    processed_result.data.rows.len().min(limit_val as usize)
                } else {
                    processed_result.data.rows.len()
                };
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

                let mut final_result = processed_result;
                if let Some(ref _column_filter) = columns {}

                if let Some(limit_val) = limit {
                    final_result.data.rows = final_result
                        .data
                        .rows
                        .into_iter()
                        .take(limit_val as usize)
                        .collect();
                }

                match format.as_str() {
                    "json" => match serde_json::to_string_pretty(&final_result) {
                        Ok(json_output) => println!("{}", json_output),
                        Err(e) => {
                            eprintln!("Error serializing to JSON: {}", e);
                            return Err(AppError::Cli(CliError::InvalidArguments(format!(
                                "Failed to serialize result to JSON: {}",
                                e
                            ))));
                        }
                    },
                    "csv" => {
                        // CSV output
                        print_verbose(verbose, "Rendering CSV output");

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
                            print_verbose(verbose, "Using full display mode");
                            let rendered_table =
                                table_display.render_query_result(&final_result)?;
                            println!("{}", rendered_table);
                        } else if no_fullscreen {
                            // Simple pagination without interactive features
                            print_verbose(verbose, "Using simple pagination mode");
                            let rendered_table = table_display
                                .render_query_result_with_limit(&final_result, Some(page_size))?;
                            println!("{}", rendered_table);
                        } else {
                            // Interactive mode - Full interactive display based on original implementation
                            print_verbose(verbose, "Using full interactive mode with crossterm");

                            let interactive_display = InteractiveDisplay::new();
                            interactive_display
                                .display_query_result_pagination(
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
                print_verbose(
                    verbose,
                    &format!(
                        "Attempting question list command - Search: {:?}, Limit: {}, Collection: {:?}",
                        search, limit, collection
                    ),
                );

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
                    interactive_display
                        .display_question_list_pagination(&questions, limit as usize)
                        .await?;
                }
                Ok(())
            }
        }
    }
}
