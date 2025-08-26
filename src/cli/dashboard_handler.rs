use crate::api::client::MetabaseClient;
use crate::cli::interactive_display::InteractiveDisplay;
use crate::cli::main_types::DashboardCommands;
use crate::core::services::dashboard_service::DashboardService;
use crate::core::services::types::ListParams;
use crate::display::{OperationStatus, ProgressSpinner, display_status};
use crate::error::{AppError, CliError};
use std::sync::Arc;

/// Handler for dashboard commands
#[derive(Default)]
pub struct DashboardHandler;

impl DashboardHandler {
    pub fn new() -> Self {
        Self
    }

    /// Print verbose message if verbose mode is enabled
    fn print_verbose(verbose: bool, msg: &str) {
        if verbose {
            println!("Verbose: {}", msg);
        }
    }

    pub async fn handle_dashboard_commands(
        &self,
        client: &MetabaseClient,
        command: &DashboardCommands,
        verbose: bool,
    ) -> Result<(), AppError> {
        let dashboard_service = DashboardService::new(Arc::new(client.clone()));

        match command {
            DashboardCommands::List {
                search,
                limit,
                format,
            } => {
                Self::print_verbose(
                    verbose,
                    &format!(
                        "Attempting dashboard list command - Search: {:?}, Limit: {}, Format: {}",
                        search, limit, format
                    ),
                );

                // Show progress while fetching dashboards
                let mut spinner = ProgressSpinner::new("Fetching dashboards...".to_string());
                spinner.start();

                // Use service layer to get dashboards
                let params = ListParams {
                    search: search.clone(),
                    limit: Some(*limit),
                    collection: None,
                    offset: None,
                };

                let dashboards = dashboard_service.list(params).await.map_err(|e| {
                    AppError::Service(crate::error::ServiceError::ConfigService {
                        message: e.to_string(),
                    })
                })?;

                spinner.stop(Some("✅ Dashboards fetched successfully"));

                // Display results
                if dashboards.is_empty() {
                    display_status("Dashboard search", OperationStatus::Warning);
                    println!("No dashboards found matching the criteria.");
                } else {
                    display_status(
                        &format!("Retrieved {} dashboards", dashboards.len()),
                        OperationStatus::Success,
                    );

                    // Format output based on format parameter
                    match format.as_str() {
                        "json" => {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&dashboards).map_err(|e| {
                                    AppError::Cli(CliError::InvalidArguments(format!(
                                        "JSON serialization error: {}",
                                        e
                                    )))
                                })?
                            );
                        }
                        _ => {
                            // Interactive table format for consistency with question list
                            let interactive_display = InteractiveDisplay::new();
                            interactive_display
                                .display_dashboard_list_pagination(&dashboards, *limit as usize)
                                .await?;
                        }
                    }
                }
                Ok(())
            }
            DashboardCommands::Show { id, format } => {
                Self::print_verbose(verbose, &format!("Showing dashboard with ID: {}", id));

                let mut spinner = ProgressSpinner::new(format!("Fetching dashboard {}...", id));
                spinner.start();

                let dashboard = dashboard_service.show(*id).await.map_err(|e| {
                    AppError::Service(crate::error::ServiceError::ConfigService {
                        message: e.to_string(),
                    })
                })?;

                spinner.stop(Some("✅ Dashboard fetched successfully"));

                match format.as_str() {
                    "json" => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&dashboard).map_err(|e| AppError::Cli(
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
                            .display_dashboard_details_interactive(&dashboard)
                            .await?;
                    }
                }
                Ok(())
            }
            DashboardCommands::Cards { id, format } => {
                Self::print_verbose(verbose, &format!("Fetching cards for dashboard ID: {}", id));

                let mut spinner =
                    ProgressSpinner::new(format!("Fetching dashboard cards for {}...", id));
                spinner.start();

                let cards = dashboard_service.get_cards(*id).await.map_err(|e| {
                    AppError::Service(crate::error::ServiceError::ConfigService {
                        message: e.to_string(),
                    })
                })?;

                spinner.stop(Some("✅ Dashboard cards fetched successfully"));

                if cards.is_empty() {
                    display_status("Dashboard cards", OperationStatus::Warning);
                    println!("No cards found for dashboard {}.", id);
                } else {
                    display_status(
                        &format!("Retrieved {} cards", cards.len()),
                        OperationStatus::Success,
                    );

                    match format.as_str() {
                        "json" => {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&cards).map_err(|e| AppError::Cli(
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
                            let page_size = 20; // Default page size for dashboard cards display
                            interactive_display
                                .display_dashboard_cards_interactive(&cards, *id, page_size)
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
    fn test_dashboard_handler_creation() {
        let handler = DashboardHandler::new();
        // Basic test to verify handler can be created
        assert_eq!(std::mem::size_of_val(&handler), 0); // Zero-sized struct
    }
}
