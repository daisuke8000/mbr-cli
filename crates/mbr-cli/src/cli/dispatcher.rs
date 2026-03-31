use crate::cli::command_handlers;
use crate::cli::main_types::{Commands, ConfigCommands};
use crate::cli::output::{
    ConfigSetOutput, LoginOutput, LogoutOutput, OutputFormat, SessionInfo, StatusOutput,
    print_json, resolve_format,
};
use mbr_core::api::client::MetabaseClient;
use mbr_core::error::{AppError, AuthError, CliError};
use mbr_core::storage::config::Config;
use mbr_core::storage::credentials::{
    Session, delete_session, get_credentials, load_session, now_iso8601, save_session,
};
use mbr_core::utils::logging::print_verbose;

pub struct Dispatcher {
    config: Config,
    verbose: bool,
    use_colors: bool,
    json_mode: bool,
}

impl Dispatcher {
    fn log_verbose(&self, msg: &str) {
        print_verbose(self.verbose, msg);
    }

    pub fn new(config: Config, verbose: bool, use_colors: bool, json_mode: bool) -> Self {
        Self {
            config,
            verbose,
            use_colors,
            json_mode,
        }
    }

    fn get_url(&self) -> Result<String, AppError> {
        self.config
            .get_url()
            .map(|cow| cow.into_owned())
            .ok_or_else(|| {
                AppError::Cli(CliError::InvalidArguments(
                    "Metabase URL is not configured. Use 'mbr-cli config set-url <url>' or set MBR_URL environment variable".to_string(),
                ))
            })
    }

    /// Create an authenticated MetabaseClient from stored session.
    fn create_client(&self) -> Result<MetabaseClient, AppError> {
        let url = self.get_url()?;
        if let Some(session) = load_session() {
            if session.url == url {
                self.log_verbose("Creating client with stored session token");
                return Ok(MetabaseClient::with_session_token(
                    url,
                    session.session_token,
                )?);
            }
            self.log_verbose("Stored session URL does not match current URL, ignoring session");
        }
        self.log_verbose("No valid session found, creating unauthenticated client");
        Ok(MetabaseClient::new(url)?)
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

        eprintln!("Logging in to {}...", url);

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

        if self.json_mode {
            print_json(&LoginOutput {
                success: true,
                username,
                url,
            });
        } else {
            eprintln!("Login successful ({})", username);
        }
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
                if self.json_mode {
                    print_json(&LogoutOutput { success: true });
                } else {
                    eprintln!("Logged out successfully");
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("Warning: {}", e);
                if self.json_mode {
                    print_json(&LogoutOutput { success: true });
                } else {
                    eprintln!("Logged out successfully");
                }
                Ok(())
            }
        }
    }

    /// Handle the `mbr status` command.
    fn handle_status(&self, format: OutputFormat) -> Result<(), AppError> {
        let format = resolve_format(self.json_mode, format);

        let url = self.config.get_url().map(|cow| cow.into_owned());
        let session = load_session();

        match format {
            OutputFormat::Json => {
                let output = StatusOutput {
                    url,
                    session: session.map(|s| SessionInfo {
                        username: s.username,
                        created_at: s.created_at,
                    }),
                };
                print_json(&output);
            }
            _ => {
                println!("Current Configuration:");
                println!("=====================");

                if let Some(ref url_val) = url {
                    println!("URL: {}", url_val);
                } else {
                    println!("URL: Not configured");
                }

                if let Some(session) = session {
                    println!("Session: Logged in ({})", session.username);
                } else {
                    println!("Session: Not logged in");
                }
            }
        }

        Ok(())
    }

    /// Attempt auto re-login using environment variables.
    async fn try_auto_relogin(&self) -> Option<MetabaseClient> {
        let (username, password) = get_credentials()?;
        let url = self.get_url().ok()?;
        self.log_verbose("Session expired, attempting auto re-login...");
        eprintln!("Session expired, re-authenticating...");
        let token = MetabaseClient::login(&url, &username, &password)
            .await
            .ok()?;
        let session = Session {
            session_token: token.clone(),
            url: url.clone(),
            username,
            created_at: now_iso8601(),
        };
        save_session(&session).ok()?;
        eprintln!("Re-authenticated successfully");
        MetabaseClient::with_session_token(url, token).ok()
    }

    fn is_unauthorized(err: &AppError) -> bool {
        matches!(
            err,
            AppError::Api(mbr_core::error::ApiError::Unauthorized { .. })
        )
    }

    /// Run a handler with auto-relogin on 401.
    async fn with_auto_relogin<F, Fut>(&self, handler: F) -> Result<(), AppError>
    where
        F: Fn(MetabaseClient) -> Fut,
        Fut: std::future::Future<Output = Result<(), AppError>>,
    {
        let client = self.create_client()?;
        match handler(client).await {
            Err(ref e) if Self::is_unauthorized(e) => {
                if let Some(new_client) = self.try_auto_relogin().await {
                    handler(new_client).await
                } else {
                    Err(AppError::Auth(AuthError::SessionExpired))
                }
            }
            other => other,
        }
    }

    pub async fn dispatch(
        &self,
        command: Commands,
        config_dir: Option<&str>,
    ) -> Result<(), AppError> {
        match command {
            Commands::Login => self.handle_login().await,
            Commands::Logout => self.handle_logout().await,
            Commands::Status { format } => self.handle_status(format),

            Commands::Config { command } => match command {
                ConfigCommands::SetUrl { url, format } => {
                    self.handle_config_set_url_with_dir(url, format, config_dir)
                }
                ConfigCommands::Validate { format } => {
                    let fmt = resolve_format(self.json_mode, format);
                    let use_colors = self.use_colors;
                    self.with_auto_relogin(|client| async move {
                        command_handlers::handle_config_validate(&client, fmt, use_colors).await
                    })
                    .await
                }
            },

            Commands::Queries {
                search,
                limit,
                collection,
                format,
            } => {
                let fmt = resolve_format(self.json_mode, format);
                let use_colors = self.use_colors;
                self.with_auto_relogin(|client| {
                    let search = search.clone();
                    let collection = collection.clone();
                    async move {
                        command_handlers::handle_queries(
                            &client, search, limit, collection, fmt, use_colors,
                        )
                        .await
                    }
                })
                .await
            }

            Commands::Run {
                id,
                param,
                format,
                limit,
                full,
                no_fullscreen,
                offset,
                page_size,
            } => {
                let fmt = resolve_format(self.json_mode, format);
                let use_colors = self.use_colors;
                self.with_auto_relogin(|client| {
                    let param = param.clone();
                    async move {
                        command_handlers::handle_run(
                            &client,
                            id,
                            param,
                            fmt,
                            limit,
                            full,
                            no_fullscreen,
                            offset,
                            page_size,
                            use_colors,
                        )
                        .await
                    }
                })
                .await
            }

            Commands::Collections { format } => {
                let fmt = resolve_format(self.json_mode, format);
                let use_colors = self.use_colors;
                self.with_auto_relogin(|client| async move {
                    command_handlers::handle_collections(&client, fmt, use_colors).await
                })
                .await
            }

            Commands::Databases { format } => {
                let fmt = resolve_format(self.json_mode, format);
                let use_colors = self.use_colors;
                self.with_auto_relogin(|client| async move {
                    command_handlers::handle_databases(&client, fmt, use_colors).await
                })
                .await
            }

            Commands::Tables {
                database_id,
                schema,
                format,
            } => {
                let fmt = resolve_format(self.json_mode, format);
                let use_colors = self.use_colors;
                self.with_auto_relogin(|client| {
                    let schema = schema.clone();
                    async move {
                        command_handlers::handle_tables(
                            &client,
                            database_id,
                            schema,
                            fmt,
                            use_colors,
                        )
                        .await
                    }
                })
                .await
            }
        }
    }

    fn handle_config_set_url_with_dir(
        &self,
        url: String,
        format: OutputFormat,
        config_dir: Option<&str>,
    ) -> Result<(), AppError> {
        let format = resolve_format(self.json_mode, format);

        mbr_core::utils::validation::validate_url(&url)?;

        let mut config = self.config.clone();
        config.set_url(url.clone());

        let config_path = config_dir.map(|dir| std::path::PathBuf::from(dir).join("config.toml"));
        config.save(config_path)?;

        match format {
            OutputFormat::Json => {
                print_json(&ConfigSetOutput { success: true, url });
            }
            _ => {
                println!("URL set to: {}", url);
                eprintln!("Configuration saved successfully.");
            }
        }

        Ok(())
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
