use crate::cli::interactive_display::InteractiveDisplay;
use crate::cli::output::{ConfigValidateOutput, OutputFormat, ValidateUserInfo, print_json};
use mbr_core::api::client::MetabaseClient;
use mbr_core::display::{
    OperationStatus, ProgressSpinner, TableDisplay, TableHeaderInfoBuilder, display_status,
};
use mbr_core::error::{AppError, CliError};
use mbr_core::storage::credentials::load_session;
use mbr_core::utils::data::OffsetManager;

/// Handle the `queries` command — list available questions.
pub async fn handle_queries(
    client: &MetabaseClient,
    search: Option<String>,
    limit: u32,
    collection: Option<String>,
    format: OutputFormat,
    _use_colors: bool,
) -> Result<(), AppError> {
    let mut spinner = ProgressSpinner::new("Fetching questions...".to_string());
    spinner.start();

    let questions = client
        .list_questions(search.as_deref(), Some(limit), collection.as_deref())
        .await?;

    spinner.stop(Some("Questions fetched successfully"));

    if questions.is_empty() {
        display_status("Question search", OperationStatus::Warning);
        match format {
            OutputFormat::Json => {
                print_json(&serde_json::json!([]));
            }
            _ => {
                println!("No questions found matching the criteria.");
            }
        }
        return Ok(());
    }

    display_status(
        &format!("Retrieved {} questions", questions.len()),
        OperationStatus::Success,
    );

    match format {
        OutputFormat::Json => {
            print_json(&questions);
        }
        OutputFormat::Csv => {
            println!("id,name,collection_id,description");
            for q in &questions {
                println!(
                    "{},{},{},{}",
                    q.id,
                    q.name,
                    q.collection_id.map(|id| id.to_string()).unwrap_or_default(),
                    q.description.as_deref().unwrap_or("")
                );
            }
        }
        OutputFormat::Table => {
            let interactive_display = InteractiveDisplay::new();
            interactive_display
                .display_question_list_pagination(&questions, limit as usize)
                .await?;
        }
    }

    Ok(())
}

/// Handle the `run` command — execute a question by ID.
#[allow(clippy::too_many_arguments)]
pub async fn handle_run(
    client: &MetabaseClient,
    id: u32,
    param: Vec<String>,
    format: OutputFormat,
    limit: u32,
    full: bool,
    no_fullscreen: bool,
    offset: Option<usize>,
    page_size: usize,
    use_colors: bool,
) -> Result<(), AppError> {
    // Convert parameters from Vec<String> to HashMap<String, String>
    let parameters = if param.is_empty() {
        None
    } else {
        let mut param_map = std::collections::HashMap::new();
        for param_str in &param {
            if let Some((key, value)) = param_str.split_once('=') {
                param_map.insert(key.to_string(), value.to_string());
            } else {
                eprintln!(
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

    let mut spinner = ProgressSpinner::new(format!("Executing question {}...", id));
    spinner.start();

    let result = client.execute_question(id, parameters).await?;
    spinner.stop(Some("Question execution completed"));

    let original_row_count = result.data.rows.len();
    let mut processed_result = result;

    // Apply offset if specified
    if let Some(offset_val) = offset
        && offset_val > 0
    {
        let offset_manager = OffsetManager::new(Some(offset_val));
        processed_result = offset_manager.apply_offset(&processed_result)?;
    }

    let table_display = TableDisplay::new().with_colors(use_colors);

    let display_start = offset.map(|o| o + 1).unwrap_or(1);
    let limit_for_display = if full { None } else { Some(limit) };
    let actual_displayed_rows = if let Some(limit_val) = limit_for_display {
        processed_result.data.rows.len().min(limit_val as usize)
    } else {
        processed_result.data.rows.len()
    };
    let display_end = if actual_displayed_rows > 0 {
        display_start + actual_displayed_rows - 1
    } else {
        display_start
    };

    let mut final_result = processed_result;

    if !full {
        final_result.data.rows = final_result
            .data
            .rows
            .into_iter()
            .take(limit as usize)
            .collect();
    }

    match format {
        OutputFormat::Json => {
            print_json(&final_result);
        }
        OutputFormat::Csv => {
            let headers: Vec<String> = final_result
                .data
                .cols
                .iter()
                .map(|col| col.display_name.clone())
                .collect();
            println!("{}", headers.join(","));

            for row in &final_result.data.rows {
                let csv_row: Vec<String> = row
                    .iter()
                    .map(|cell| table_display.format_cell_value(cell))
                    .collect();
                println!("{}", csv_row.join(","));
            }
        }
        OutputFormat::Table => {
            let header_info = TableHeaderInfoBuilder::new()
                .data_source("Question execution result".to_string())
                .source_id(id)
                .total_records(original_row_count)
                .display_range(display_start, display_end)
                .offset(offset.unwrap_or(0))
                .build();

            print!(
                "{}",
                table_display.render_comprehensive_header(&header_info)
            );

            if full {
                let rendered_table = table_display.render_query_result(&final_result)?;
                println!("{}", rendered_table);
            } else if no_fullscreen {
                let rendered_table =
                    table_display.render_query_result_with_limit(&final_result, Some(page_size))?;
                println!("{}", rendered_table);
            } else {
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

/// Handle the `collections` command — list all collections.
pub async fn handle_collections(
    client: &MetabaseClient,
    format: OutputFormat,
    use_colors: bool,
) -> Result<(), AppError> {
    let mut spinner = ProgressSpinner::new("Fetching collections...".to_string());
    spinner.start();

    let collections = client.list_collections().await?;
    spinner.stop(Some("Collections fetched successfully"));

    if collections.is_empty() {
        display_status("Collection search", OperationStatus::Warning);
        match format {
            OutputFormat::Json => {
                print_json(&serde_json::json!([]));
            }
            _ => {
                println!("No collections found.");
            }
        }
        return Ok(());
    }

    display_status(
        &format!("Retrieved {} collections", collections.len()),
        OperationStatus::Success,
    );

    match format {
        OutputFormat::Json => {
            print_json(&collections);
        }
        OutputFormat::Csv => {
            println!("id,name,description,personal_owner_id");
            for collection in &collections {
                println!(
                    "{},{},{},{}",
                    collection.id.map(|id| id.to_string()).unwrap_or_default(),
                    collection.name,
                    collection.description.as_deref().unwrap_or(""),
                    collection
                        .personal_owner_id
                        .map(|id| id.to_string())
                        .unwrap_or_default()
                );
            }
        }
        OutputFormat::Table => {
            let table_display = TableDisplay::new().with_colors(use_colors);
            let headers = vec!["ID", "Name", "Description", "Personal"];
            let rows: Vec<Vec<String>> = collections
                .iter()
                .map(|c| {
                    vec![
                        c.id.map(|id| id.to_string()).unwrap_or("-".to_string()),
                        c.name.clone(),
                        c.description.as_deref().unwrap_or("-").to_string(),
                        if c.personal_owner_id.is_some() {
                            "Yes"
                        } else {
                            "No"
                        }
                        .to_string(),
                    ]
                })
                .collect();
            let table = table_display.render_simple_table(&headers, &rows);
            println!("{}", table);
        }
    }

    Ok(())
}

/// Handle the `databases` command — list all databases.
pub async fn handle_databases(
    client: &MetabaseClient,
    format: OutputFormat,
    use_colors: bool,
) -> Result<(), AppError> {
    let mut spinner = ProgressSpinner::new("Fetching databases...".to_string());
    spinner.start();

    let databases = client.list_databases().await?;
    spinner.stop(Some("Databases fetched successfully"));

    if databases.is_empty() {
        display_status("Database search", OperationStatus::Warning);
        match format {
            OutputFormat::Json => {
                print_json(&serde_json::json!([]));
            }
            _ => {
                println!("No databases found.");
            }
        }
        return Ok(());
    }

    display_status(
        &format!("Retrieved {} databases", databases.len()),
        OperationStatus::Success,
    );

    match format {
        OutputFormat::Json => {
            print_json(&databases);
        }
        OutputFormat::Csv => {
            println!("id,name,engine,is_sample");
            for db in &databases {
                println!(
                    "{},{},{},{}",
                    db.id,
                    db.name,
                    db.engine.as_deref().unwrap_or(""),
                    db.is_sample
                );
            }
        }
        OutputFormat::Table => {
            let table_display = TableDisplay::new().with_colors(use_colors);
            let headers = vec!["ID", "Name", "Engine", "Sample"];
            let rows: Vec<Vec<String>> = databases
                .iter()
                .map(|db| {
                    vec![
                        db.id.to_string(),
                        db.name.clone(),
                        db.engine.as_deref().unwrap_or("-").to_string(),
                        if db.is_sample { "Yes" } else { "No" }.to_string(),
                    ]
                })
                .collect();
            let table = table_display.render_simple_table(&headers, &rows);
            println!("{}", table);
        }
    }

    Ok(())
}

/// Handle the `tables` command — list tables in a database schema.
pub async fn handle_tables(
    client: &MetabaseClient,
    database_id: u32,
    schema: String,
    format: OutputFormat,
    use_colors: bool,
) -> Result<(), AppError> {
    let mut spinner =
        ProgressSpinner::new(format!("Fetching tables for database {}...", database_id));
    spinner.start();

    let tables = client.list_tables(database_id, &schema).await?;
    spinner.stop(Some("Tables fetched successfully"));

    if tables.is_empty() {
        display_status("Table search", OperationStatus::Warning);
        match format {
            OutputFormat::Json => {
                print_json(&serde_json::json!([]));
            }
            _ => {
                println!(
                    "No tables found in schema '{}' of database {}.",
                    schema, database_id
                );
            }
        }
        return Ok(());
    }

    display_status(
        &format!("Retrieved {} tables", tables.len()),
        OperationStatus::Success,
    );

    match format {
        OutputFormat::Json => {
            print_json(&tables);
        }
        OutputFormat::Csv => {
            println!("id,name,schema,display_name,description");
            for t in &tables {
                println!(
                    "{},{},{},{},{}",
                    t.id,
                    t.name,
                    t.schema.as_deref().unwrap_or(""),
                    t.display_name.as_deref().unwrap_or(""),
                    t.description.as_deref().unwrap_or("")
                );
            }
        }
        OutputFormat::Table => {
            let table_display = TableDisplay::new().with_colors(use_colors);
            let headers = vec!["ID", "Name", "Schema", "Display Name", "Description"];
            let rows: Vec<Vec<String>> = tables
                .iter()
                .map(|t| {
                    vec![
                        t.id.to_string(),
                        t.name.clone(),
                        t.schema.as_deref().unwrap_or("-").to_string(),
                        t.display_name.as_deref().unwrap_or("-").to_string(),
                        t.description.as_deref().unwrap_or("-").to_string(),
                    ]
                })
                .collect();
            let table = table_display.render_simple_table(&headers, &rows);
            println!("{}", table);
        }
    }

    Ok(())
}

/// Handle the `config validate` command.
pub async fn handle_config_validate(
    client: &MetabaseClient,
    format: OutputFormat,
    _use_colors: bool,
) -> Result<(), AppError> {
    let session = load_session();
    if session.is_none() {
        match format {
            OutputFormat::Json => {
                print_json(&ConfigValidateOutput {
                    valid: false,
                    user: None,
                    error: Some("Not logged in".to_string()),
                });
                return Ok(());
            }
            _ => {
                println!("Not logged in.");
                println!();
                println!("To authenticate, run:");
                println!("  mbr-cli login");
                println!();
                println!("Or set environment variables:");
                println!("  export MBR_USERNAME=\"your_username\"");
                println!("  export MBR_PASSWORD=\"your_password\"");
            }
        }
        return Err(AppError::Cli(CliError::AuthRequired {
            message: "Not logged in".to_string(),
            hint: "Run 'mbr-cli login' to authenticate".to_string(),
        }));
    }

    if !client.is_authenticated() {
        match format {
            OutputFormat::Json => {
                print_json(&ConfigValidateOutput {
                    valid: false,
                    user: None,
                    error: Some("Client is not authenticated".to_string()),
                });
                return Ok(());
            }
            _ => {
                println!("Session token is not configured properly.");
            }
        }
        return Err(AppError::Cli(CliError::AuthRequired {
            message: "Client is not authenticated".to_string(),
            hint: "Run 'mbr-cli login' to re-authenticate".to_string(),
        }));
    }

    let mut spinner = ProgressSpinner::new("Validating session...".to_string());
    spinner.start();

    match client.get_current_user().await {
        Ok(user) => {
            spinner.stop(Some("Session validated successfully"));
            let name = user.common_name.clone().or(user.first_name.clone());

            match format {
                OutputFormat::Json => {
                    print_json(&ConfigValidateOutput {
                        valid: true,
                        user: Some(ValidateUserInfo {
                            id: user.id,
                            email: user.email,
                            name,
                            is_superuser: user.is_superuser,
                        }),
                        error: None,
                    });
                }
                _ => {
                    println!();
                    println!("Authentication Status:");
                    println!("=====================");
                    println!("Connected to Metabase");
                    println!();
                    println!("User Information:");
                    println!("  ID: {}", user.id);
                    println!("  Email: {}", user.email);
                    if let Some(n) = name {
                        println!("  Name: {}", n);
                    }
                    if let Some(is_superuser) = user.is_superuser {
                        println!("  Admin: {}", if is_superuser { "Yes" } else { "No" });
                    }
                }
            }
            Ok(())
        }
        Err(e) => {
            spinner.stop(Some("Session validation failed"));
            match format {
                OutputFormat::Json => {
                    print_json(&ConfigValidateOutput {
                        valid: false,
                        user: None,
                        error: Some(format!("{}", e)),
                    });
                    Ok(())
                }
                _ => {
                    println!();
                    println!("Failed to validate session: {}", e);
                    println!();
                    println!("Possible causes:");
                    println!("  - Session has expired");
                    println!("  - Metabase server is unreachable");
                    println!();
                    println!("Try running 'mbr-cli login' to re-authenticate.");
                    Err(e)
                }
            }
        }
    }
}
