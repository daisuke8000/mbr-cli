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
    pub fn collect() -> Result<Self, AppError> {
        // Username
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
            username: username.trim().to_string(),
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
