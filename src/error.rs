use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("CliError: {0}")]
    Cli(#[from] CliError),
    #[error("ApiError: {0}")]
    Api(#[from] ApiError),
    #[error("ConfigError: {0}")]
    Config(#[from] ConfigError),
    #[error("AuthError: {0}")]
    Auth(#[from] AuthError),
    #[error("StorageError: {0}")]
    Storage(#[from] StorageError),
    #[error("DisplayError: {0}")]
    Display(#[from] DisplayError),
    #[error("QuestionError: {0}")]
    Question(#[from] QuestionError),
    #[error("ServiceError: {0}")]
    Service(#[from] ServiceError),
    #[error("UtilsError: {0}")]
    Utils(#[from] UtilsError),
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
    #[error("Command not implemented: {command}")]
    NotImplemented { command: String },
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
pub enum AuthError {
    #[error("Login failed: Invalid credentials")]
    InvalidCredentials,
    #[error("Session expired or invalid")]
    SessionInvalid,
    #[error("Login failed")]
    LoginFailed,
    #[error("API key authentication failed")]
    ApiKeyInvalid,
}

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Keyring error: {0}")]
    KeyringError(String),
    #[error("File I/O error at {path}: {source}")]
    FileIo {
        path: String,
        source: std::io::Error,
    },
    #[error("Session storage failed")]
    SessionStorageFailed,
    #[error("Configuration save failed")]
    ConfigSaveFailed,
    #[error("Configuration parse error: {message}")]
    ConfigParseError { message: String },
    #[error("Configuration directory not found")]
    ConfigDirNotFound,
}

#[derive(Error, Debug)]
pub enum DisplayError {
    #[error("Table formatting failed: {0}")]
    TableFormat(String),
    #[error("Terminal output error: {0}")]
    TerminalOutput(String),
    #[error("Pagination error: {0}")]
    Pagination(String),
}

#[derive(Error, Debug)]
pub enum QuestionError {
    #[error("Question {id} not found")]
    NotFound { id: u32 },
    #[error("Question execution failed for ID {id}: {reason}")]
    ExecutionFailed { id: u32, reason: String },
    #[error("Invalid parameter {parameter}")]
    InvalidParameter { parameter: String },
    #[error("Question list retrieval failed with status {status_code}")]
    ListFailed { status_code: u16 },
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

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Authentication service error: {message}")]
    AuthService { message: String },
    #[error("Configuration service error: {message}")]
    ConfigService { message: String },
    #[error("Question service error: {message}")]
    QuestionService { message: String },
}

#[derive(Error, Debug)]
pub enum UtilsError {
    #[error("Validation error: {message}")]
    Validation { message: String },
    #[error("Memory calculation error: {message}")]
    Memory { message: String },
    #[error("Data processing error: {message}")]
    DataProcessing { message: String },
    #[error("Input processing error: {message}")]
    InputProcessing { message: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorSeverity {
    Critical,
    High,
    Medium,
    Low,
}

impl ErrorSeverity {
    pub fn emoji(&self) -> &'static str {
        match self {
            ErrorSeverity::Critical => "🚨",
            ErrorSeverity::High => "❌",
            ErrorSeverity::Medium => "⚠️",
            ErrorSeverity::Low => "ℹ️",
        }
    }
}

impl AppError {
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            AppError::Cli(_) => ErrorSeverity::Medium,
            AppError::Api(api_error) => match api_error {
                ApiError::Unauthorized { .. } => ErrorSeverity::High,
                ApiError::Timeout { .. } => ErrorSeverity::Medium,
                ApiError::Http { status, .. } if *status >= 500 => ErrorSeverity::High,
                _ => ErrorSeverity::Medium,
            },
            AppError::Config(_) => ErrorSeverity::High,
            AppError::Auth(_) => ErrorSeverity::High,
            AppError::Storage(_) => ErrorSeverity::Medium,
            AppError::Display(_) => ErrorSeverity::Low,
            AppError::Question(_) => ErrorSeverity::Medium,
            AppError::Service(service_error) => match service_error {
                ServiceError::AuthService { .. } => ErrorSeverity::High,
                ServiceError::ConfigService { .. } => ErrorSeverity::Medium,
                ServiceError::QuestionService { .. } => ErrorSeverity::Medium,
            },
            AppError::Utils(_) => ErrorSeverity::Low,
        }
    }

    pub fn display_friendly(&self) -> String {
        match self {
            AppError::Auth(AuthError::InvalidCredentials) => "Invalid credentials".to_string(),
            AppError::Auth(AuthError::SessionInvalid) => "Session expired or invalid".to_string(),
            AppError::Config(ConfigError::FileNotFound { .. }) => {
                "configuration file not found: ".to_string()
            }
            AppError::Question(QuestionError::NotFound { id }) => {
                format!("Question {} not found", id)
            }
            _ => format!("{}", self),
        }
    }

    pub fn troubleshooting_hint(&self) -> Option<String> {
        match self {
            AppError::Auth(AuthError::InvalidCredentials | AuthError::SessionInvalid) => {
                Some("'mbr-cli auth login' try again".to_string())
            }
            AppError::Config(ConfigError::FileNotFound { .. }) => {
                Some("config set <field> <value> to set a configuration value".to_string())
            }
            AppError::Api(ApiError::Timeout { .. }) => {
                Some("Check your internet or Metabase connection and try again".to_string())
            }
            AppError::Question(QuestionError::NotFound { .. }) => {
                Some("'mbr-cli question list' to see a list of available questions".to_string())
            }
            _ => None,
        }
    }
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

    #[test]
    fn test_service_error_display() {
        let service_err = ServiceError::AuthService {
            message: "Authentication failed".to_string(),
        };
        assert_eq!(
            format!("{}", service_err),
            "Authentication service error: Authentication failed"
        );

        let service_err = ServiceError::ConfigService {
            message: "Configuration invalid".to_string(),
        };
        assert_eq!(
            format!("{}", service_err),
            "Configuration service error: Configuration invalid"
        );

        let service_err = ServiceError::QuestionService {
            message: "Question execution failed".to_string(),
        };
        assert_eq!(
            format!("{}", service_err),
            "Question service error: Question execution failed"
        );
    }

    #[test]
    fn test_utils_error_display() {
        let utils_err = UtilsError::Validation {
            message: "Invalid URL format".to_string(),
        };
        assert_eq!(
            format!("{}", utils_err),
            "Validation error: Invalid URL format"
        );

        let utils_err = UtilsError::Memory {
            message: "Memory allocation failed".to_string(),
        };
        assert_eq!(
            format!("{}", utils_err),
            "Memory calculation error: Memory allocation failed"
        );

        let utils_err = UtilsError::DataProcessing {
            message: "Data parsing failed".to_string(),
        };
        assert_eq!(
            format!("{}", utils_err),
            "Data processing error: Data parsing failed"
        );

        let utils_err = UtilsError::InputProcessing {
            message: "Input validation failed".to_string(),
        };
        assert_eq!(
            format!("{}", utils_err),
            "Input processing error: Input validation failed"
        );
    }

    #[test]
    fn test_app_error_service_utils_integration() {
        let app_err = AppError::Service(ServiceError::AuthService {
            message: "Authentication failed".to_string(),
        });
        assert_eq!(app_err.severity(), ErrorSeverity::High);
        assert_eq!(
            format!("{}", app_err),
            "ServiceError: Authentication service error: Authentication failed"
        );

        let app_err = AppError::Utils(UtilsError::Validation {
            message: "Invalid input".to_string(),
        });
        assert_eq!(app_err.severity(), ErrorSeverity::Low);
        assert_eq!(
            format!("{}", app_err),
            "UtilsError: Validation error: Invalid input"
        );
    }
}
