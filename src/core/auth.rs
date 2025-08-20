use crate::error::{AppError, CliError};
use rpassword::read_password;
use std::io::{self, Write};

/// User login credentials input handler
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

impl LoginInput {
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
