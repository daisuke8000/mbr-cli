use crate::error::{AppError, CliError};
use rpassword::read_password;
use std::env;
use std::io::{self, Write};

/// User login credentials input handler
#[derive(Clone)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

impl LoginInput {
    /// Create LoginInput from command line arguments with environment variable fallback
    /// Priority: CLI args > environment variables > interactive input
    pub fn from_args_or_env(
        cli_username: Option<String>,
        cli_password: Option<String>,
        profile_email: Option<&str>,
    ) -> Result<Self, AppError> {
        // Check for username: CLI args > ENV > profile > interactive
        let username = if let Some(username) = cli_username {
            println!("Using username from command line argument");
            username
        } else if let Ok(env_username) = env::var("MBR_USERNAME") {
            if !env_username.is_empty() {
                println!("Using username from MBR_USERNAME environment variable");
                env_username
            } else {
                return Err(AppError::Cli(CliError::InvalidArguments(
                    "MBR_USERNAME environment variable is empty".to_string(),
                )));
            }
        } else if let Some(email) = profile_email {
            println!("Using email from profile: {}", email);
            email.to_string()
        } else {
            // Interactive prompt for username
            print!("Username: ");
            io::stdout().flush().map_err(|e| {
                AppError::Cli(CliError::InvalidArguments(format!(
                    "Failed to flush stdout: {}",
                    e
                )))
            })?;

            let mut username = String::new();
            io::stdin().read_line(&mut username).map_err(|e| {
                AppError::Cli(CliError::InvalidArguments(format!(
                    "Failed to read username: {}",
                    e
                )))
            })?;
            username.trim().to_string()
        };

        // Check for password: CLI args > ENV > interactive
        let password = if let Some(password) = cli_password {
            println!("Using password from command line argument");
            password
        } else if let Ok(env_password) = env::var("MBR_PASSWORD") {
            if !env_password.is_empty() {
                println!("Using password from MBR_PASSWORD environment variable");
                env_password
            } else {
                return Err(AppError::Cli(CliError::InvalidArguments(
                    "MBR_PASSWORD environment variable is empty".to_string(),
                )));
            }
        } else {
            // Interactive prompt for password
            print!("Password: ");
            io::stdout().flush().map_err(|e| {
                AppError::Cli(CliError::InvalidArguments(format!(
                    "Failed to flush stdout: {}",
                    e
                )))
            })?;

            read_password().map_err(|e| {
                AppError::Cli(CliError::InvalidArguments(format!(
                    "Failed to read password: {}",
                    e
                )))
            })?
        };

        Ok(Self {
            username,
            password: password.trim().to_string(),
        })
    }

    /// Collect login credentials from interactive input
    /// If profile_email is provided, only password will be prompted
    pub fn collect(profile_email: Option<&str>) -> Result<Self, AppError> {
        let username = if let Some(email) = profile_email {
            // Use email from profile
            println!("Using email from profile: {}", email);
            email.to_string()
        } else {
            // Prompt for username
            print!("Username: ");
            io::stdout().flush().map_err(|e| {
                AppError::Cli(CliError::InvalidArguments(format!(
                    "Failed to flush stdout: {}",
                    e
                )))
            })?;

            let mut username = String::new();
            io::stdin().read_line(&mut username).map_err(|e| {
                AppError::Cli(CliError::InvalidArguments(format!(
                    "Failed to read username: {}",
                    e
                )))
            })?;
            username.trim().to_string()
        };

        // Password
        print!("Password: ");
        io::stdout().flush().map_err(|e| {
            AppError::Cli(CliError::InvalidArguments(format!(
                "Failed to flush stdout: {}",
                e
            )))
        })?;

        let password = read_password().map_err(|e| {
            AppError::Cli(CliError::InvalidArguments(format!(
                "Failed to read password: {}",
                e
            )))
        })?;

        Ok(Self {
            username,
            password: password.trim().to_string(),
        })
    }

    /// Validate that credentials are not empty
    pub fn validate(&self) -> Result<(), AppError> {
        if self.username.is_empty() {
            return Err(AppError::Cli(CliError::InvalidArguments(
                "Username cannot be empty".to_string(),
            )));
        }
        if self.password.is_empty() {
            return Err(AppError::Cli(CliError::InvalidArguments(
                "Password cannot be empty".to_string(),
            )));
        }
        Ok(())
    }
}
