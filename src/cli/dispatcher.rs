use crate::api::client::MetabaseClient;
use crate::cli::main_types::{AuthCommands, Commands, ConfigCommands, QuestionCommands};
use crate::core::auth::LoginInput;
use crate::core::services::auth_service::AuthService;
use crate::core::services::config_service::ConfigService;
use crate::display::{
    OffsetManager, OperationStatus, ProgressSpinner, TableDisplay, TableHeaderInfoBuilder,
    display_status,
};
use crate::error::{AppError, CliError};
use crate::storage::config::{Config, Profile};
use crate::storage::credentials::{AuthMode, Credentials};
use crossterm::execute;

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

    /// Based on original implementation: RAW mode + Alternate Screen + interactive display with scroll functionality
    async fn display_interactive_pagination(
        &self,
        result: &crate::api::models::QueryResult,
        page_size: usize,
        initial_offset: Option<usize>,
        no_fullscreen: bool,
        question_id: u32,
        question_name: &str,
    ) -> Result<(), AppError> {
        use crossterm::{
            cursor, event,
            event::{Event, KeyCode, KeyEvent, KeyModifiers},
            execute,
            style::{Color, Print, ResetColor, SetForegroundColor},
            terminal::{
                Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
                enable_raw_mode, size,
            },
        };
        use std::io::{self, Write};

        // RAII cleanup structures
        struct RawModeCleanup;
        impl Drop for RawModeCleanup {
            fn drop(&mut self) {
                let _ = disable_raw_mode();
            }
        }

        struct ScreenCleanup;
        impl Drop for ScreenCleanup {
            fn drop(&mut self) {
                let _ = execute!(io::stdout(), LeaveAlternateScreen);
            }
        }

        if no_fullscreen {
            // Simple mode fallback
            let display = crate::display::table::TableDisplay::new();
            let table_output = display.render_query_result(result)?;
            println!("{}", table_output);
            return Ok(());
        }

        // Full screen mode - RAW mode + Alternate Screen + scroll
        match enable_raw_mode() {
            Ok(()) => {
                let _cleanup = RawModeCleanup;
                execute!(io::stdout(), EnterAlternateScreen).ok();
                let _screen_cleanup = ScreenCleanup;

                // Get terminal size
                let (_terminal_width, terminal_height) = size().unwrap_or((80, 24));

                // Pagination state
                let total_rows = result.data.rows.len();
                let base_offset = 0; // Use 0 here since the result is already offset-adjusted
                let available_rows = total_rows.saturating_sub(base_offset);
                let total_pages = if available_rows == 0 {
                    1
                } else {
                    available_rows.div_ceil(page_size)
                }; // Total pages considering offset
                let mut current_page = 1; // Initial display always starts from page 1

                // Table renderer for display
                let display = crate::display::table::TableDisplay::new();

                // Scroll state (for scrolling within table)
                let mut scroll_offset = 0;
                // Reserve 8 lines: header space (5 lines) + prompt space (3 lines)
                let available_height = terminal_height.saturating_sub(8) as usize;

                loop {
                    // Get current page data (considering offset)
                    let start_row = base_offset + (current_page - 1) * page_size;
                    let end_row = (start_row + page_size).min(total_rows);

                    // Create QueryResult limited to current page data
                    let page_rows = if start_row < total_rows {
                        result.data.rows[start_row..end_row].to_vec()
                    } else {
                        vec![]
                    };

                    let page_result = crate::api::models::QueryResult {
                        data: crate::api::models::QueryData {
                            cols: result.data.cols.clone(),
                            rows: page_rows,
                        },
                    };

                    // Generate table for current page
                    let page_table_output = display.render_query_result(&page_result)?;
                    let table_lines: Vec<&str> = page_table_output.lines().collect();

                    // Clear entire screen
                    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();

                    // Display header (fixed)
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Cyan),
                        Print(format!("Question {}: {}", question_id, question_name)),
                        ResetColor,
                        Print("\r\n"),
                        SetForegroundColor(Color::Yellow),
                        Print(format!(
                            "Page {}/{} | Showing rows {}-{} of {} total (available: {}, offset: {}) | Page size: {}",
                            current_page,
                            total_pages,
                            initial_offset.unwrap_or(0) + (current_page - 1) * page_size + 1,  // Correct user display range start
                            initial_offset.unwrap_or(0) + (current_page - 1) * page_size + (end_row - start_row),  // Correct user display range end
                            total_rows,
                            available_rows,
                            initial_offset.unwrap_or(0),
                            page_size
                        )),
                        ResetColor,
                        Print("\r\n\r\n")
                    ).ok();

                    // Display content within scroll range
                    let total_lines = table_lines.len();
                    let start_line = scroll_offset.min(total_lines);
                    let end_line = (start_line + available_height).min(total_lines);

                    if start_line < total_lines {
                        let display_lines = &table_lines[start_line..end_line];
                        for line in display_lines {
                            println!("{}\r", line);
                        }
                    }

                    // Clear bottom of screen (prevent leftover characters)
                    execute!(io::stdout(), Clear(ClearType::FromCursorDown)).ok();

                    // Display prompt (fixed at bottom)
                    execute!(
                        io::stdout(),
                        cursor::MoveTo(0, terminal_height - 2),
                        SetForegroundColor(Color::Green),
                        Print("Controls: ↑↓/jk=scroll | n/p=page | Home/End | q=quit | h=help"),
                        ResetColor
                    )
                    .ok();

                    io::stdout().flush().ok();

                    // Key input processing
                    if let Ok(Event::Key(KeyEvent {
                        code, modifiers, ..
                    })) = event::read()
                    {
                        match code {
                            // Exit
                            KeyCode::Char('q') | KeyCode::Char('Q') => break,
                            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                                break;
                            }
                            KeyCode::Esc => break,

                            // Scroll (line by line)
                            KeyCode::Up | KeyCode::Char('k') => {
                                scroll_offset = scroll_offset.saturating_sub(1);
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                let max_offset = total_lines.saturating_sub(available_height);
                                scroll_offset = (scroll_offset + 1).min(max_offset);
                            }

                            // Page navigation (data pages)
                            KeyCode::Char('n') => {
                                if current_page < total_pages {
                                    current_page += 1;
                                    scroll_offset = 0; // Reset scroll position for new page
                                }
                            }
                            KeyCode::Char('p') => {
                                if current_page > 1 {
                                    current_page -= 1;
                                    scroll_offset = 0; // Reset scroll position for new page
                                }
                            }

                            // Scroll movement (within page)
                            KeyCode::PageUp => {
                                scroll_offset = scroll_offset.saturating_sub(available_height);
                            }
                            KeyCode::PageDown => {
                                let max_offset = total_lines.saturating_sub(available_height);
                                scroll_offset = (scroll_offset + available_height).min(max_offset);
                            }

                            // First/last (page navigation)
                            KeyCode::Home => {
                                current_page = 1;
                                scroll_offset = 0;
                            }
                            KeyCode::End => {
                                current_page = total_pages.max(1);
                                scroll_offset = 0;
                            }

                            // Show help
                            KeyCode::Char('h') | KeyCode::Char('H') => {
                                execute!(
                                    io::stdout(),
                                    Clear(ClearType::All),
                                    cursor::MoveTo(0, 0),
                                    SetForegroundColor(Color::Cyan),
                                    Print("Keyboard Navigation Help"),
                                    ResetColor,
                                    Print("\r\n\r\n"),
                                    Print("Page Navigation:\r\n"),
                                    Print("  n           : Next page\r\n"),
                                    Print("  p           : Previous page\r\n"),
                                    Print("  Home        : First page\r\n"),
                                    Print("  End         : Last page\r\n"),
                                    Print("\r\n"),
                                    Print("Scroll Controls (within page):\r\n"),
                                    Print("  ↑, k        : Scroll up (1 line)\r\n"),
                                    Print("  ↓, j        : Scroll down (1 line)\r\n"),
                                    Print("  Page Up     : Scroll up (page)\r\n"),
                                    Print("  Page Down   : Scroll down (page)\r\n"),
                                    Print("\r\n"),
                                    Print("Other Controls:\r\n"),
                                    Print("  q, Q, Esc  : Quit\r\n"),
                                    Print("  Ctrl+C     : Force quit\r\n"),
                                    Print("  h, H       : Show this help\r\n"),
                                    Print("\r\n"),
                                    SetForegroundColor(Color::Yellow),
                                    Print("Press any key to continue..."),
                                    ResetColor
                                )
                                .ok();
                                io::stdout().flush().ok();
                                event::read().ok();
                            }

                            _ => {} // Ignore invalid keys
                        }
                    }
                }
            }
            Err(_) => {
                // Fallback when RAW mode fails
                println!("Warning: Could not enable full-screen mode, falling back to simple mode");
                let display = crate::display::table::TableDisplay::new();
                let table_output = display.render_query_result(result)?;
                println!("{}", table_output);
            }
        }

        Ok(())
    }

    /// RAW mode + Alternate Screen + pagination display for Question List
    async fn display_interactive_question_list(
        &self,
        questions: &[crate::api::models::Question],
        page_size: usize,
    ) -> Result<(), AppError> {
        use crossterm::{
            cursor, event,
            event::{Event, KeyCode, KeyEvent, KeyModifiers},
            execute,
            style::{Color, Print, ResetColor, SetForegroundColor},
            terminal::{
                Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
                enable_raw_mode, size,
            },
        };
        use std::io::{self, Write};

        // RAII cleanup structures
        struct RawModeCleanup;
        impl Drop for RawModeCleanup {
            fn drop(&mut self) {
                let _ = disable_raw_mode();
            }
        }

        struct ScreenCleanup;
        impl Drop for ScreenCleanup {
            fn drop(&mut self) {
                let _ = execute!(io::stdout(), LeaveAlternateScreen);
            }
        }

        // Full screen mode - RAW mode + Alternate Screen + pagination (always used)
        match enable_raw_mode() {
            Ok(()) => {
                let _cleanup = RawModeCleanup;
                execute!(io::stdout(), EnterAlternateScreen).ok();
                let _screen_cleanup = ScreenCleanup;

                // Get terminal size
                let (_terminal_width, terminal_height) = size().unwrap_or((80, 24));

                // Pagination state
                let total_questions = questions.len();
                let total_pages = if total_questions == 0 {
                    1
                } else {
                    total_questions.div_ceil(page_size)
                }; // Ceiling calculation, minimum 1 page
                let mut current_page = 1;

                // Table renderer for display
                let display = crate::display::table::TableDisplay::new();

                // Scroll state (for scrolling within table)
                let mut scroll_offset = 0;
                // Reserve 8 lines: header space (5 lines) + prompt space (3 lines)
                let available_height = terminal_height.saturating_sub(8) as usize;

                loop {
                    // Get current page data
                    let start_idx = (current_page - 1) * page_size;
                    let end_idx = (start_idx + page_size).min(total_questions);

                    // Current page question data
                    let page_questions = if start_idx < total_questions {
                        &questions[start_idx..end_idx]
                    } else {
                        &[]
                    };

                    // Pre-calculate table row count
                    let total_lines = if total_questions == 0 {
                        0
                    } else {
                        let page_table_output = display.render_question_list(page_questions)?;
                        page_table_output.lines().count()
                    };

                    // Clear entire screen
                    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();

                    // Display header (fixed)
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Cyan),
                        Print("Question List"),
                        ResetColor,
                        Print("\r\n"),
                        SetForegroundColor(Color::Yellow),
                        Print(format!(
                            "Page {}/{} | Showing questions {}-{} of {} | Page size: {}",
                            current_page,
                            total_pages,
                            if total_questions == 0 {
                                0
                            } else {
                                start_idx + 1
                            },
                            if total_questions == 0 { 0 } else { end_idx },
                            total_questions,
                            page_size
                        )),
                        ResetColor,
                        Print("\r\n\r\n")
                    )
                    .ok();

                    if total_questions == 0 {
                        // Message for empty question list
                        execute!(
                            io::stdout(),
                            SetForegroundColor(Color::Yellow),
                            Print("No questions found matching the criteria."),
                            ResetColor,
                            Print("\r\n\r\n"),
                            Print("Please modify your search criteria or create a new question.")
                        )
                        .ok();
                    } else {
                        // Generate table for current page (already calculated)
                        let page_table_output = display.render_question_list(page_questions)?;
                        let table_lines: Vec<&str> = page_table_output.lines().collect();

                        // Display content within scroll range
                        let start_line = scroll_offset.min(total_lines);
                        let end_line = (start_line + available_height).min(total_lines);

                        if start_line < total_lines {
                            let display_lines = &table_lines[start_line..end_line];
                            for line in display_lines {
                                println!("{}\r", line);
                            }
                        }
                    }

                    // Clear bottom of screen (prevent leftover characters)
                    execute!(io::stdout(), Clear(ClearType::FromCursorDown)).ok();

                    // Display prompt (fixed at bottom)
                    execute!(
                        io::stdout(),
                        cursor::MoveTo(0, terminal_height - 2),
                        SetForegroundColor(Color::Green),
                        Print("Controls: ↑↓/jk=scroll | n/p=page | Home/End | q=quit | h=help"),
                        ResetColor
                    )
                    .ok();

                    io::stdout().flush().ok();

                    // Key input processing
                    if let Ok(Event::Key(KeyEvent {
                        code, modifiers, ..
                    })) = event::read()
                    {
                        match code {
                            // Exit
                            KeyCode::Char('q') | KeyCode::Char('Q') => break,
                            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                                break;
                            }
                            KeyCode::Esc => break,

                            // Scroll (line by line)
                            KeyCode::Up | KeyCode::Char('k') => {
                                scroll_offset = scroll_offset.saturating_sub(1);
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                let max_offset = total_lines.saturating_sub(available_height);
                                scroll_offset = (scroll_offset + 1).min(max_offset);
                            }

                            // Page navigation (data pages)
                            KeyCode::Char('n') => {
                                if current_page < total_pages {
                                    current_page += 1;
                                    scroll_offset = 0; // Reset scroll position for new page
                                }
                            }
                            KeyCode::Char('p') => {
                                if current_page > 1 {
                                    current_page -= 1;
                                    scroll_offset = 0; // Reset scroll position for new page
                                }
                            }

                            // Scroll movement (within page)
                            KeyCode::PageUp => {
                                scroll_offset = scroll_offset.saturating_sub(available_height);
                            }
                            KeyCode::PageDown => {
                                let max_offset = total_lines.saturating_sub(available_height);
                                scroll_offset = (scroll_offset + available_height).min(max_offset);
                            }

                            // First/last (page navigation)
                            KeyCode::Home => {
                                current_page = 1;
                                scroll_offset = 0;
                            }
                            KeyCode::End => {
                                current_page = total_pages.max(1);
                                scroll_offset = 0;
                            }

                            // Show help
                            KeyCode::Char('h') | KeyCode::Char('H') => {
                                execute!(
                                    io::stdout(),
                                    Clear(ClearType::All),
                                    cursor::MoveTo(0, 0),
                                    SetForegroundColor(Color::Cyan),
                                    Print("Question List - Keyboard Navigation Help"),
                                    ResetColor,
                                    Print("\r\n\r\n"),
                                    Print("Page Navigation:\r\n"),
                                    Print("  n           : Next page\r\n"),
                                    Print("  p           : Previous page\r\n"),
                                    Print("  Home        : First page\r\n"),
                                    Print("  End         : Last page\r\n"),
                                    Print("\r\n"),
                                    Print("Scroll Controls (within page):\r\n"),
                                    Print("  ↑, k        : Scroll up (1 line)\r\n"),
                                    Print("  ↓, j        : Scroll down (1 line)\r\n"),
                                    Print("  Page Up     : Scroll up (page)\r\n"),
                                    Print("  Page Down   : Scroll down (page)\r\n"),
                                    Print("\r\n"),
                                    Print("Other Controls:\r\n"),
                                    Print("  q, Q, Esc  : Quit\r\n"),
                                    Print("  Ctrl+C     : Force quit\r\n"),
                                    Print("  h, H       : Show this help\r\n"),
                                    Print("\r\n"),
                                    SetForegroundColor(Color::Yellow),
                                    Print("Press any key to continue..."),
                                    ResetColor
                                )
                                .ok();
                                io::stdout().flush().ok();
                                event::read().ok();
                            }

                            _ => {} // Ignore invalid keys
                        }
                    }
                }
            }
            Err(_) => {
                // Fallback when RAW mode fails
                println!("Warning: Could not enable full-screen mode, falling back to simple mode");
                let display = crate::display::table::TableDisplay::new();
                let table_output = display.render_question_list(questions)?;
                println!("{}", table_output);
            }
        }

        Ok(())
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
                format: "table".to_string(),
                limit: Some(20),
                full: false,
                no_fullscreen: false,
                offset: None,
                columns: None,
                page_size: 20,
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
    async fn test_dispatcher_creates_auth_service() {
        let d = create_test_dispatcher(true);
        let profile = d.get_current_profile().expect("Profile should exist");
        
        // Test should pass after refactoring to use AuthService
        let auth_service = d.create_auth_service(profile);
        assert!(auth_service.is_ok(), "Should create AuthService successfully");
        
        let service = auth_service.unwrap();
        let status = service.get_auth_status();
        assert_eq!(status.profile_name, "test");
    }

    #[tokio::test]
    async fn test_auth_commands_use_auth_service() {
        let d = create_test_dispatcher(true);
        
        // Test that auth status command uses AuthService
        let result = d.handle_auth_command(AuthCommands::Status).await;
        assert!(result.is_ok(), "Auth status should work with AuthService");
        
        // Test that logout uses AuthService
        let result = d.handle_auth_command(AuthCommands::Logout).await;
        assert!(result.is_ok(), "Auth logout should work with AuthService");
    }

    #[tokio::test]
    async fn test_dispatcher_creates_config_service() {
        let d = create_test_dispatcher(true);
        
        // Test should pass after refactoring to use ConfigService
        let config_service = d.create_config_service();
        let profiles = config_service.list_profiles();
        assert_eq!(profiles.len(), 1);
        assert!(profiles.iter().any(|(name, _)| *name == "test"));
    }

    #[tokio::test]
    async fn test_config_commands_use_config_service() {
        let d = create_test_dispatcher(true);
        
        // Test that config show command uses ConfigService
        let result = d.handle_config_command(ConfigCommands::Show).await;
        assert!(result.is_ok(), "Config show should work with ConfigService");
        
        // Test that config set uses ConfigService
        let result = d.handle_config_command(ConfigCommands::Set {
            profile: "test".to_string(),
            field: "url".to_string(),
            value: "http://localhost:3000".to_string(),
        }).await;
        match &result {
            Ok(_) => { /* test passes */ }
            Err(e) => {
                println!("Config set error: {}", e);
                panic!("Config set should work with ConfigService: {:?}", e);
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
