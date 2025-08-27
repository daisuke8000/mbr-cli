use crate::api::client::MetabaseClient;
use crate::cli::interactive_display::InteractiveDisplay;
use crate::cli::main_types::CollectionCommands;
use crate::core::services::collection_service::CollectionService;
use crate::display::{OperationStatus, ProgressSpinner, display_status};
use crate::error::{AppError, CliError};
use std::sync::Arc;

/// Handler for collection commands
#[derive(Default)]
pub struct CollectionHandler;

impl CollectionHandler {
    pub fn new() -> Self {
        Self
    }

    /// Print verbose message if verbose mode is enabled
    fn print_verbose(verbose: bool, msg: &str) {
        if verbose {
            println!("Verbose: {}", msg);
        }
    }

    pub async fn handle_collection_commands(
        &self,
        client: &MetabaseClient,
        command: &CollectionCommands,
        verbose: bool,
    ) -> Result<(), AppError> {
        let collection_service = CollectionService::new(Arc::new(client.clone()));

        match command {
            CollectionCommands::List { tree, format } => {
                Self::print_verbose(
                    verbose,
                    &format!(
                        "Attempting collection list command - Tree: {}, Format: {}",
                        tree, format
                    ),
                );

                // Show progress while fetching collections
                let mut spinner = ProgressSpinner::new("Fetching collections...".to_string());
                spinner.start();

                let collections = collection_service.list(*tree).await.map_err(|e| {
                    Self::print_verbose(verbose, &format!("Collection service error: {}", e));
                    AppError::Service(crate::error::ServiceError::ConfigService {
                        message: e.to_string(),
                    })
                })?;

                Self::print_verbose(verbose, &format!("Retrieved {} collections from service", collections.len()));

                spinner.stop(Some("✅ Collections fetched successfully"));

                // Display results
                if collections.is_empty() {
                    display_status("Collection search", OperationStatus::Warning);
                    println!("No collections found.");
                } else {
                    display_status(
                        &format!("Retrieved {} collections", collections.len()),
                        OperationStatus::Success,
                    );

                    // Format output based on format parameter
                    match format.as_str() {
                        "json" => {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&collections).map_err(|e| {
                                    AppError::Cli(CliError::InvalidArguments(format!(
                                        "JSON serialization error: {}",
                                        e
                                    )))
                                })?
                            );
                        }
                        _ => {
                            // Interactive table format for consistency with dashboard list
                            let interactive_display = InteractiveDisplay::new();
                            interactive_display
                                .display_collection_list_pagination(&collections, 20)
                                .await?;
                        }
                    }
                }
                Ok(())
            }
            CollectionCommands::Show { id, format } => {
                Self::print_verbose(verbose, &format!("Showing collection with ID: {}", id));

                let mut spinner = ProgressSpinner::new(format!("Fetching collection {}...", id));
                spinner.start();

                let collection = collection_service.show(*id).await.map_err(|e| {
                    AppError::Service(crate::error::ServiceError::ConfigService {
                        message: e.to_string(),
                    })
                })?;

                spinner.stop(Some("✅ Collection fetched successfully"));

                match format.as_str() {
                    "json" => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&collection).map_err(|e| AppError::Cli(
                                CliError::InvalidArguments(format!(
                                    "JSON serialization error: {}",
                                    e
                                ))
                            ))?
                        );
                    }
                    _ => {
                        // Interactive table format for consistency with other commands
                        let interactive_display = InteractiveDisplay::new();
                        interactive_display
                            .display_collection_details_interactive(&collection)
                            .await?;
                    }
                }
                Ok(())
            }
            CollectionCommands::Stats { id, format } => {
                Self::print_verbose(verbose, &format!("Fetching statistics for collection ID: {}", id));

                let mut spinner =
                    ProgressSpinner::new(format!("Calculating collection statistics for {}...", id));
                spinner.start();

                let stats = collection_service.get_stats(*id).await.map_err(|e| {
                    AppError::Service(crate::error::ServiceError::ConfigService {
                        message: e.to_string(),
                    })
                })?;

                spinner.stop(Some("✅ Collection statistics calculated successfully"));

                if stats.item_count == 0 {
                    display_status("Collection statistics", OperationStatus::Warning);
                    println!("No items found in collection {}.", id);
                } else {
                    display_status(
                        &format!("Statistics calculated for collection {}", id),
                        OperationStatus::Success,
                    );

                    match format.as_str() {
                        "json" => {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&stats).map_err(|e| AppError::Cli(
                                    CliError::InvalidArguments(format!(
                                        "JSON serialization error: {}",
                                        e
                                    ))
                                ))?
                            );
                        }
                        _ => {
                            // Interactive table format for consistency with other commands
                            let interactive_display = InteractiveDisplay::new();
                            interactive_display
                                .display_collection_stats_interactive(&stats, *id)
                                .await?;
                        }
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

    #[test]
    fn test_collection_handler_creation() {
        let handler = CollectionHandler::new();
        // Basic test to verify handler can be created
        assert_eq!(std::mem::size_of_val(&handler), 0); // Zero-sized struct
    }
}