use crate::cli::command_handlers::{
    CollectionHandler, ConfigHandler, DatabaseHandler, QueryHandler,
};
use crate::cli::main_types::Commands;
use mbr_core::api::client::MetabaseClient;
use mbr_core::core::services::config_service::ConfigService;
use mbr_core::error::{AppError, AuthError, CliError};
use mbr_core::storage::config::Config;
use mbr_core::storage::credentials::{
    delete_session, get_credentials, load_session, now_iso8601, save_session, Session,
};
use mbr_core::utils::logging::print_verbose;

pub struct Dispatcher {
    config: Config,
    verbose: bool,
}

impl Dispatcher {
    fn log_verbose(&self, msg: &str) {
        print_verbose(self.verbose, msg);
    }

    pub fn new(config: Config, verbose: bool) -> Self {
        Self { config, verbose }
    }

    fn get_url(&self) -> Result<String, AppError> {
        self.config.get_url().ok_or_else(|| {
            AppError::Cli(CliError::InvalidArguments(
                "Metabase URL is not configured. Use 'mbr-cli config set --url <url>' or set MBR_URL environment variable".to_string(),
            ))
        })
    }

    /// Create an authenticated MetabaseClient from stored session.
    fn create_client(&self) -> Result<MetabaseClient, AppError> {
        let url = self.get_url()?;
        if let Some(session) = load_session() {
            if session.url == url {
                self.log_verbose("Creating client with stored session token");
                return Ok(MetabaseClient::with_session_token(url, session.session_token)?);
            }
            self.log_verbose("Stored session URL does not match current URL, ignoring session");
        }
        self.log_verbose("No valid session found, creating unauthenticated client");
        Ok(MetabaseClient::new(url)?)
    }

    fn create_config_service(&self) -> ConfigService {
        ConfigService::new(self.config.clone())
    }

    /// Handle the `mbr login` command.
    async fn handle_login(&self) -> Result<(), AppError> {
        let url = self.get_url()?;

        let (username, password) = if let Some(creds) = get_credentials() {
            self.log_verbose("Using credentials from environment variables");
            creds
        } else {
            self.log_verbose("Prompting for credentials");
            let username = prompt_username()?;
            let password = prompt_password()?;
            (username, password)
        };

        println!("Logging in to {}...", url);

        let token = MetabaseClient::login(&url, &username, &password).await?;

        let session = Session {
            session_token: token,
            url: url.clone(),
            username: username.clone(),
            created_at: now_iso8601(),
        };

        save_session(&session).map_err(|e| {
            AppError::Auth(AuthError::LoginFailed {
                message: format!("Failed to save session: {}", e),
            })
        })?;

        println!("✓ Login successful ({})", username);
        Ok(())
    }

    /// Handle the `mbr logout` command.
    async fn handle_logout(&self) -> Result<(), AppError> {
        if let Some(session) = load_session()
            && let Ok(client) =
                MetabaseClient::with_session_token(session.url, session.session_token)
        {
            let _ = client.logout().await;
        }

        match delete_session() {
            Ok(()) => {
                println!("✓ Logged out successfully");
                Ok(())
            }
            Err(e) => {
                eprintln!("Warning: {}", e);
                println!("✓ Logged out successfully");
                Ok(())
            }
        }
    }

    /// Attempt auto re-login using environment variables.
    async fn try_auto_relogin(&self) -> Option<MetabaseClient> {
        let (username, password) = get_credentials()?;
        let url = self.get_url().ok()?;
        self.log_verbose("Session expired, attempting auto re-login...");
        eprintln!("ℹ Session expired, re-authenticating...");
        let token = MetabaseClient::login(&url, &username, &password).await.ok()?;
        let session = Session {
            session_token: token.clone(),
            url: url.clone(),
            username,
            created_at: now_iso8601(),
        };
        save_session(&session).ok()?;
        eprintln!("✓ Re-authenticated successfully");
        MetabaseClient::with_session_token(url, token).ok()
    }

    fn is_unauthorized(err: &AppError) -> bool {
        matches!(err, AppError::Api(mbr_core::error::ApiError::Unauthorized { .. }))
    }

    pub async fn dispatch(&self, command: Commands) -> Result<(), AppError> {
        match command {
            Commands::Login => self.handle_login().await,
            Commands::Logout => self.handle_logout().await,
            Commands::Config { command } => {
                let handler = ConfigHandler::new();
                let mut config_service = self.create_config_service();
                let client = match self.create_client() {
                    Ok(c) => c,
                    Err(_) => MetabaseClient::new("http://localhost:3000".to_string())?,
                };
                handler
                    .handle(command, &mut config_service, client, self.verbose)
                    .await
            }
            Commands::Query(args) => {
                let handler = QueryHandler::new();
                let client = self.create_client()?;
                match handler.handle(args.clone(), client, self.verbose).await {
                    Err(ref e) if Self::is_unauthorized(e) => {
                        if let Some(new_client) = self.try_auto_relogin().await {
                            handler.handle(args, new_client, self.verbose).await
                        } else {
                            Err(AppError::Auth(AuthError::SessionExpired))
                        }
                    }
                    other => other,
                }
            }
            Commands::Collection { command } => {
                let handler = CollectionHandler::new();
                let client = self.create_client()?;
                match handler.handle(command.clone(), client, self.verbose).await {
                    Err(ref e) if Self::is_unauthorized(e) => {
                        if let Some(new_client) = self.try_auto_relogin().await {
                            handler.handle(command, new_client, self.verbose).await
                        } else {
                            Err(AppError::Auth(AuthError::SessionExpired))
                        }
                    }
                    other => other,
                }
            }
            Commands::Database { command } => {
                let handler = DatabaseHandler::new();
                let client = self.create_client()?;
                match handler.handle(command.clone(), client, self.verbose).await {
                    Err(ref e) if Self::is_unauthorized(e) => {
                        if let Some(new_client) = self.try_auto_relogin().await {
                            handler.handle(command, new_client, self.verbose).await
                        } else {
                            Err(AppError::Auth(AuthError::SessionExpired))
                        }
                    }
                    other => other,
                }
            }
        }
    }
}

fn prompt_username() -> Result<String, AppError> {
    eprint!("Metabase Username: ");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).map_err(|_| {
        AppError::Auth(AuthError::LoginFailed {
            message: "Failed to read username".to_string(),
        })
    })?;
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        return Err(AppError::Auth(AuthError::LoginFailed {
            message: "Username cannot be empty".to_string(),
        }));
    }
    Ok(trimmed)
}

fn prompt_password() -> Result<String, AppError> {
    rpassword::prompt_password_stderr("Metabase Password: ").map_err(|_| {
        AppError::Auth(AuthError::LoginFailed {
            message: "Failed to read password".to_string(),
        })
    })
}
