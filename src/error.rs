use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("CliError: {0}")]
    Cli(#[from] CliError),
    #[error("ApiError: {0}")]
    Api(#[from] ApiError),
    #[error("ConfigError: {0}")]
    Config(#[from] ConfigError),
}

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Authentication required")]
    AuthRequired {
        message: String,
        hint: String,
        available_profiles: Vec<String>,
    },
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Request timed out after {timeout_secs}s")]
    Timeout { timeout_secs: u64, endpoint: String },

    #[error("HTTP error: {status} {message}")]
    Http {
        status: u16,
        endpoint: String,
        message: String,
    },

    #[error("Authentication failed")]
    Unauthorized {
        status: u16,
        endpoint: String,
        server_message: String,
    },
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String, hint: String },

    #[error("Configuration field '{field}' is missing")]
    MissingField { field: String, field_type: String },

    #[error("Invalid configuration value for '{field}': {value}")]
    InvalidValue {
        field: String,
        value: String,
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_error_display() {
        let cli_err = CliError::InvalidArguments("invalid arguments".to_string());
        assert_eq!(
            format!("{}", cli_err),
            "Invalid arguments: invalid arguments"
        );
        let cli_err = CliError::AuthRequired {
            message: "message".to_string(),
            hint: "hint".to_string(),
            available_profiles: vec!["profile1".to_string(), "profile2".to_string()],
        };
        assert!(matches!(cli_err, CliError::AuthRequired { .. }));
        if let CliError::AuthRequired {
            message,
            hint,
            available_profiles,
        } = cli_err
        {
            assert_eq!(message, "message");
            assert_eq!(hint, "hint");
            assert_eq!(
                available_profiles,
                vec!["profile1".to_string(), "profile2".to_string()]
            );
        }
    }

    #[test]
    fn test_config_error_display() {
        let config_err = ConfigError::FileNotFound {
            path: "config.toml".to_string(),
            hint: "hint".to_string(),
        };
        assert!(matches!(config_err, ConfigError::FileNotFound { .. }));
        if let ConfigError::FileNotFound { path, hint } = config_err {
            assert_eq!(path, "config.toml");
            assert_eq!(hint, "hint");
        };

        let config_err = ConfigError::MissingField {
            field: "field".to_string(),
            field_type: "type".to_string(),
        };
        assert!(matches!(config_err, ConfigError::MissingField { .. }));
        if let ConfigError::MissingField { field, field_type } = config_err {
            assert_eq!(field, "field");
            assert_eq!(field_type, "type");
        };

        let config_err = ConfigError::InvalidValue {
            field: "field".to_string(),
            value: "value".to_string(),
            reason: "reason".to_string(),
        };
        assert!(matches!(config_err, ConfigError::InvalidValue { .. }));
        if let ConfigError::InvalidValue {
            field,
            value,
            reason,
        } = config_err
        {
            assert_eq!(field, "field");
            assert_eq!(value, "value");
            assert_eq!(reason, "reason");
        }
    }

    #[test]
    fn test_api_error_display() {
        let api_err = ApiError::Unauthorized {
            status: 401,
            endpoint: "endpoint".to_string(),
            server_message: "message".to_string(),
        };
        assert!(matches!(api_err, ApiError::Unauthorized { .. }));
        if let ApiError::Unauthorized {
            status,
            endpoint,
            server_message,
        } = api_err
        {
            assert_eq!(status, 401);
            assert_eq!(endpoint, "endpoint");
            assert_eq!(server_message, "message");
        };

        let api_err = ApiError::Timeout {
            timeout_secs: 10,
            endpoint: "endpoint".to_string(),
        };
        assert!(matches!(api_err, ApiError::Timeout { .. }));
        if let ApiError::Timeout {
            timeout_secs,
            endpoint,
        } = api_err
        {
            assert_eq!(timeout_secs, 10);
            assert_eq!(endpoint, "endpoint");
        };

        let api_err = ApiError::Http {
            status: 400,
            endpoint: "endpoint".to_string(),
            message: "message".to_string(),
        };
        assert!(matches!(api_err, ApiError::Http { .. }));
        if let ApiError::Http {
            status,
            endpoint,
            message,
        } = api_err
        {
            assert_eq!(status, 400);
            assert_eq!(endpoint, "endpoint");
            assert_eq!(message, "message");
        }
    }

    #[test]
    fn test_app_error_display_cli() {
        let app_err = AppError::Cli(CliError::InvalidArguments("invalid arguments".to_string()));
        assert_eq!(
            format!("{}", app_err),
            "CliError: Invalid arguments: invalid arguments"
        );
        let app_err = AppError::Cli(CliError::AuthRequired {
            message: "message".to_string(),
            hint: "hint".to_string(),
            available_profiles: vec!["profile1".to_string(), "profile2".to_string()],
        });
        assert!(matches!(
            app_err,
            AppError::Cli(CliError::AuthRequired { .. })
        ));
        if let AppError::Cli(CliError::AuthRequired {
            message,
            hint,
            available_profiles,
        }) = app_err
        {
            assert_eq!(message, "message");
            assert_eq!(hint, "hint");
            assert_eq!(
                available_profiles,
                vec!["profile1".to_string(), "profile2".to_string()]
            );
        }
    }

    #[test]
    fn test_app_error_display_api() {
        let app_err = AppError::Api(ApiError::Unauthorized {
            status: 401,
            endpoint: "endpoint".to_string(),
            server_message: "message".to_string(),
        });
        assert!(matches!(
            app_err,
            AppError::Api(ApiError::Unauthorized { .. })
        ));
        if let AppError::Api(ApiError::Unauthorized {
            status,
            endpoint,
            server_message,
        }) = app_err
        {
            assert_eq!(status, 401);
            assert_eq!(endpoint, "endpoint");
            assert_eq!(server_message, "message");
        }

        let app_err = AppError::Api(ApiError::Http {
            status: 400,
            endpoint: "endpoint".to_string(),
            message: "message".to_string(),
        });
        assert!(matches!(app_err, AppError::Api(ApiError::Http { .. })));
        if let AppError::Api(ApiError::Http {
            status,
            endpoint,
            message,
        }) = app_err
        {
            assert_eq!(status, 400);
            assert_eq!(endpoint, "endpoint");
            assert_eq!(message, "message");
        }

        let app_err = AppError::Api(ApiError::Timeout {
            timeout_secs: 10,
            endpoint: "endpoint".to_string(),
        });
        assert!(matches!(app_err, AppError::Api(ApiError::Timeout { .. })));
        if let AppError::Api(ApiError::Timeout {
            timeout_secs,
            endpoint,
        }) = app_err
        {
            assert_eq!(timeout_secs, 10);
            assert_eq!(endpoint, "endpoint");
        }
    }

    #[test]
    fn test_app_error_display_config() {
        if let AppError::Config(ConfigError::FileNotFound { path, hint }) =
            AppError::Config(ConfigError::FileNotFound {
                path: "config.toml".to_string(),
                hint: "hint".to_string(),
            })
        {
            assert_eq!(path, "config.toml");
            assert_eq!(hint, "hint");
        };

        if let AppError::Config(ConfigError::MissingField { field, field_type }) =
            AppError::Config(ConfigError::MissingField {
                field: "field".to_string(),
                field_type: "type".to_string(),
            })
        {
            assert_eq!(field, "field");
            assert_eq!(field_type, "type");
        };

        if let AppError::Config(ConfigError::InvalidValue {
            field,
            value,
            reason,
        }) = AppError::Config(ConfigError::InvalidValue {
            field: "field".to_string(),
            value: "value".to_string(),
            reason: "reason".to_string(),
        }) {
            assert_eq!(field, "field");
            assert_eq!(value, "value");
            assert_eq!(reason, "reason");
        }
    }
}
