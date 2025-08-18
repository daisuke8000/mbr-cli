use crate::cli::main_types::{AuthCommands, Commands, ConfigCommands, QuestionCommands};
use crate::error::{AppError, CliError};
use crate::storage::config::Config;
use crate::storage::credentials::Credentials;

pub struct Dispatcher {
    config: Config,
    credentials: Credentials,
    verbose: bool,
}

impl Dispatcher {
    pub fn new(config: Config, credentials: Credentials, verbose: bool) -> Self {
        Self {
            config,
            credentials,
            verbose,
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
            AuthCommands::Login => {
                if self.verbose {
                    println!("Verbose: Attempting auth login command");
                }
                Err(AppError::Cli(CliError::NotImplemented {
                    command: "auth login".to_string(),
                }))
            }
            AuthCommands::Logout => {
                if self.verbose {
                    println!("Verbose: Attempting auth logout command");
                }
                Err(AppError::Cli(CliError::NotImplemented {
                    command: "auth logout".to_string(),
                }))
            }
            AuthCommands::Status => {
                if self.verbose {
                    println!("Verbose: Attempting auth status command");
                }

                // Show authentication status
                println!("Authentication Status:");
                println!("=====================");

                let auth_mode = self.credentials.get_auth_mode();
                match auth_mode {
                    crate::storage::credentials::AuthMode::APIKey => {
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
                    crate::storage::credentials::AuthMode::Session => {
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
                if self.verbose {
                    println!("Verbose: Attempting config show command");
                }

                // Show current configuration
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
                        println!("    Metabase URL: {}", profile.metabase_url);
                        if let Some(timeout) = profile.timeout_seconds {
                            println!("    Timeout: {} seconds", timeout);
                        }
                        if let Some(cache) = profile.cache_enabled {
                            println!("    Cache: {}", if cache { "enabled" } else { "disabled" });
                        }
                    }
                }

                Ok(())
            }
            ConfigCommands::Set { key, value } => {
                if self.verbose {
                    println!(
                        "Verbose: Attempting config set - key: {}, value: {}",
                        key, value
                    );
                }
                Err(AppError::Cli(CliError::NotImplemented {
                    command: format!("config set - key: {}, value: {}", key, value),
                }))
            }
        }
    }

    async fn handle_question_command(&self, commands: QuestionCommands) -> Result<(), AppError> {
        match commands {
            QuestionCommands::Execute { id, param, limit } => {
                if self.verbose {
                    println!(
                        "Verbose: Attempting question execute command - ID: {}, Params: {:?}, Limit: {:?}",
                        id, param, limit
                    );
                }
                Err(AppError::Cli(CliError::NotImplemented {
                    command: format!(
                        "question execute - ID: {}, Params: {:?}, Limit: {:?}",
                        id, param, limit
                    ),
                }))
            }
            QuestionCommands::List {
                search,
                limit,
                collection,
            } => {
                if self.verbose {
                    println!(
                        "Verbose: Attempting question list command - Search: {:?}, Limit: {}, Collection: {:?}",
                        search, limit, collection
                    );
                }
                Err(AppError::Cli(CliError::NotImplemented {
                    command: format!(
                        "question list - Search: {:?}, Limit: {}, Collection: {:?}",
                        search, limit, collection
                    ),
                }))
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
                        timeout_seconds: Some(30),
                        cache_enabled: Some(true),
                    },
                );
                profiles
            },
        };
        let creds = Credentials::new("test".to_string());
        Dispatcher::new(config, creds, verbose)
    }

    #[tokio::test]
    async fn test_dispatcher_creation() {
        let d = create_test_dispatcher(true);
        assert_eq!(d.verbose, true);
    }

    #[tokio::test]
    async fn test_auth_login_not_implemented() {
        let d = create_test_dispatcher(true);
        let result = d.handle_auth_command(AuthCommands::Login).await;
        assert!(result.is_err());
        if let Err(AppError::Cli(CliError::NotImplemented { command })) = result {
            assert_eq!(command, "auth login");
        } else {
            panic!("Expected NotImplemented error for auth login");
        }
    }

    #[tokio::test]
    async fn test_auth_logout_not_implemented() {
        let d = create_test_dispatcher(true);
        let result = d.handle_auth_command(AuthCommands::Logout).await;
        assert!(result.is_err());
        if let Err(AppError::Cli(CliError::NotImplemented { command })) = result {
            assert_eq!(command, "auth logout");
        } else {
            panic!("Expected NotImplemented error for auth logout");
        }
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
    async fn test_config_set_not_implemented() {
        let d = create_test_dispatcher(true);
        let result = d
            .handle_config_command(ConfigCommands::Set {
                key: "test".to_string(),
                value: "test".to_string(),
            })
            .await;
        assert!(result.is_err());
        if let Err(AppError::Cli(CliError::NotImplemented { command })) = result {
            assert_eq!(command, "config set - key: test, value: test");
        }
    }

    #[tokio::test]
    async fn test_question_list_not_implemented() {
        let d = create_test_dispatcher(true);
        let result = d
            .handle_question_command(QuestionCommands::List {
                search: None,
                limit: 10,
                collection: None,
            })
            .await;
        assert!(result.is_err());
        if let Err(AppError::Cli(CliError::NotImplemented { command })) = result {
            assert_eq!(
                command,
                "question list - Search: None, Limit: 10, Collection: None"
            );
        } else {
            panic!("Expected NotImplemented error for question list");
        }
    }

    #[tokio::test]
    async fn test_question_execute_not_implemented() {
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
        assert!(result.is_err());
        if let Err(AppError::Cli(CliError::NotImplemented { command })) = result {
            assert_eq!(
                command,
                "question execute - ID: 1, Params: [\"sample_param=sample_value\", \"another_param=another_value\"], Limit: Some(20)"
            );
        } else {
            panic!("Expected NotImplemented error for question execute");
        }
    }

    #[tokio::test]
    async fn test_dispatch_cmd() {
        let d = create_test_dispatcher(true);

        // Auth login should still fail (not implemented)
        let result = d
            .dispatch(Commands::Auth {
                command: AuthCommands::Login,
            })
            .await;
        assert!(result.is_err());

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
