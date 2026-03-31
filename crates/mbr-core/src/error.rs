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
    AuthRequired { message: String, hint: String },
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
    #[error("Permission denied")]
    Forbidden {
        status: u16,
        endpoint: String,
        server_message: String,
    },
}

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Not logged in")]
    NotLoggedIn,
    #[error("Session expired")]
    SessionExpired,
    #[error("Login failed: {message}")]
    LoginFailed { message: String },
}

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("File I/O error at {path}: {source}")]
    FileIo {
        path: String,
        source: std::io::Error,
    },
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
    #[error("Data processing error: {message}")]
    DataProcessing { message: String },
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
                ApiError::Forbidden { .. } => ErrorSeverity::Medium,
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

    /// Return a machine-readable error code string for programmatic error handling.
    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::Cli(e) => match e {
                CliError::AuthRequired { .. } => "CLI_AUTH_REQUIRED",
                CliError::InvalidArguments(_) => "CLI_INVALID_ARGUMENTS",
                CliError::NotImplemented { .. } => "CLI_NOT_IMPLEMENTED",
            },
            AppError::Api(e) => match e {
                ApiError::Timeout { .. } => "API_TIMEOUT",
                ApiError::Http { .. } => "API_HTTP_ERROR",
                ApiError::Unauthorized { .. } => "API_UNAUTHORIZED",
                ApiError::Forbidden { .. } => "API_FORBIDDEN",
            },
            AppError::Config(e) => match e {
                ConfigError::FileNotFound { .. } => "CONFIG_FILE_NOT_FOUND",
                ConfigError::MissingField { .. } => "CONFIG_MISSING_FIELD",
                ConfigError::InvalidValue { .. } => "CONFIG_INVALID_VALUE",
            },
            AppError::Auth(e) => match e {
                AuthError::NotLoggedIn => "AUTH_NOT_LOGGED_IN",
                AuthError::SessionExpired => "AUTH_SESSION_EXPIRED",
                AuthError::LoginFailed { .. } => "AUTH_LOGIN_FAILED",
            },
            AppError::Storage(e) => match e {
                StorageError::FileIo { .. } => "STORAGE_FILE_IO",
                StorageError::ConfigParseError { .. } => "STORAGE_CONFIG_PARSE",
                StorageError::ConfigDirNotFound => "STORAGE_CONFIG_DIR_NOT_FOUND",
            },
            AppError::Display(e) => match e {
                DisplayError::TableFormat(_) => "DISPLAY_TABLE_FORMAT",
                DisplayError::TerminalOutput(_) => "DISPLAY_TERMINAL_OUTPUT",
                DisplayError::Pagination(_) => "DISPLAY_PAGINATION",
            },
            AppError::Question(e) => match e {
                QuestionError::NotFound { .. } => "QUESTION_NOT_FOUND",
                QuestionError::ExecutionFailed { .. } => "QUESTION_EXECUTION_FAILED",
                QuestionError::InvalidParameter { .. } => "QUESTION_INVALID_PARAMETER",
                QuestionError::ListFailed { .. } => "QUESTION_LIST_FAILED",
            },
            AppError::Service(e) => match e {
                ServiceError::AuthService { .. } => "SERVICE_AUTH",
                ServiceError::ConfigService { .. } => "SERVICE_CONFIG",
                ServiceError::QuestionService { .. } => "SERVICE_QUESTION",
            },
            AppError::Utils(e) => match e {
                UtilsError::Validation { .. } => "UTILS_VALIDATION",
                UtilsError::DataProcessing { .. } => "UTILS_DATA_PROCESSING",
            },
        }
    }

    pub fn display_friendly(&self) -> String {
        match self {
            AppError::Auth(AuthError::NotLoggedIn) => "Not logged in".to_string(),
            AppError::Auth(AuthError::SessionExpired) => "Session expired".to_string(),
            AppError::Auth(AuthError::LoginFailed { message }) => {
                format!("Login failed: {}", message)
            }
            AppError::Config(ConfigError::FileNotFound { .. }) => {
                "Configuration file not found".to_string()
            }
            AppError::Question(QuestionError::NotFound { id }) => {
                format!("Question {} not found", id)
            }
            _ => format!("{}", self),
        }
    }

    pub fn troubleshooting_hint(&self) -> Option<String> {
        match self {
            AppError::Auth(AuthError::NotLoggedIn) => {
                Some("Run 'mbr-cli login' to authenticate with Metabase.\nOr set MBR_USERNAME and MBR_PASSWORD environment variables.".to_string())
            }
            AppError::Auth(AuthError::SessionExpired) => {
                Some("Your session has expired. Run 'mbr-cli login' to re-authenticate.".to_string())
            }
            AppError::Auth(AuthError::LoginFailed { .. }) => {
                Some("Check your username and password and try again.".to_string())
            }
            AppError::Config(ConfigError::FileNotFound { .. }) => {
                Some("Run 'mbr-cli config set --url <url>' to create a configuration".to_string())
            }
            AppError::Api(ApiError::Timeout { .. }) => {
                Some("Check your network connection and Metabase server availability".to_string())
            }
            AppError::Api(ApiError::Unauthorized { .. }) => {
                Some("Your session may be expired. Run 'mbr-cli login' to re-authenticate.".to_string())
            }
            AppError::Question(QuestionError::NotFound { .. }) => {
                Some("Run 'mbr-cli query --list' to see available questions".to_string())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_formats() {
        assert!(format!("{}", CliError::InvalidArguments("test".into())).contains("test"));
        assert!(
            format!(
                "{}",
                ApiError::Timeout {
                    timeout_secs: 10,
                    endpoint: "ep".into()
                }
            )
            .contains("10")
        );
        assert!(
            format!(
                "{}",
                ConfigError::FileNotFound {
                    path: "p".into(),
                    hint: "h".into()
                }
            )
            .contains("p")
        );
        assert!(
            format!(
                "{}",
                ServiceError::AuthService {
                    message: "m".into()
                }
            )
            .contains("m")
        );
        assert!(
            format!(
                "{}",
                UtilsError::Validation {
                    message: "v".into()
                }
            )
            .contains("v")
        );
    }

    #[test]
    fn test_error_variants_constructible() {
        assert!(matches!(
            CliError::AuthRequired {
                message: "".into(),
                hint: "".into(),
            },
            CliError::AuthRequired { .. }
        ));
        assert!(matches!(
            ApiError::Unauthorized {
                status: 401,
                endpoint: "".into(),
                server_message: "".into()
            },
            ApiError::Unauthorized { .. }
        ));
        assert!(matches!(
            ApiError::Http {
                status: 400,
                endpoint: "".into(),
                message: "".into()
            },
            ApiError::Http { .. }
        ));
        assert!(matches!(
            ConfigError::MissingField {
                field: "".into(),
                field_type: "".into()
            },
            ConfigError::MissingField { .. }
        ));
        assert!(matches!(
            ConfigError::InvalidValue {
                field: "".into(),
                value: "".into(),
                reason: "".into()
            },
            ConfigError::InvalidValue { .. }
        ));
    }

    #[test]
    fn test_app_error_severity() {
        assert_eq!(
            AppError::Api(ApiError::Unauthorized {
                status: 401,
                endpoint: "".into(),
                server_message: "".into()
            })
            .severity(),
            ErrorSeverity::High
        );
        assert_eq!(
            AppError::Api(ApiError::Http {
                status: 500,
                endpoint: "".into(),
                message: "".into()
            })
            .severity(),
            ErrorSeverity::High
        );
        assert_eq!(
            AppError::Api(ApiError::Http {
                status: 400,
                endpoint: "".into(),
                message: "".into()
            })
            .severity(),
            ErrorSeverity::Medium
        );
        assert_eq!(
            AppError::Config(ConfigError::FileNotFound {
                path: "".into(),
                hint: "".into()
            })
            .severity(),
            ErrorSeverity::High
        );
        assert_eq!(
            AppError::Display(DisplayError::Pagination("".into())).severity(),
            ErrorSeverity::Low
        );
        assert_eq!(
            AppError::Service(ServiceError::AuthService { message: "".into() }).severity(),
            ErrorSeverity::High
        );
    }

    #[test]
    fn test_app_error_conversion() {
        let cli: AppError = CliError::InvalidArguments("".into()).into();
        let api: AppError = ApiError::Timeout {
            timeout_secs: 1,
            endpoint: "".into(),
        }
        .into();
        let config: AppError = ConfigError::FileNotFound {
            path: "".into(),
            hint: "".into(),
        }
        .into();
        assert!(matches!(cli, AppError::Cli(_)));
        assert!(matches!(api, AppError::Api(_)));
        assert!(matches!(config, AppError::Config(_)));
    }

    #[test]
    fn test_app_error_severity_forbidden() {
        assert_eq!(
            AppError::Api(ApiError::Forbidden {
                status: 403,
                endpoint: "".into(),
                server_message: "".into()
            })
            .severity(),
            ErrorSeverity::Medium
        );
    }

    #[test]
    fn test_error_code_cli() {
        assert_eq!(
            AppError::Cli(CliError::AuthRequired {
                message: "".into(),
                hint: "".into()
            })
            .error_code(),
            "CLI_AUTH_REQUIRED"
        );
        assert_eq!(
            AppError::Cli(CliError::InvalidArguments("".into())).error_code(),
            "CLI_INVALID_ARGUMENTS"
        );
        assert_eq!(
            AppError::Cli(CliError::NotImplemented { command: "".into() }).error_code(),
            "CLI_NOT_IMPLEMENTED"
        );
    }

    #[test]
    fn test_error_code_api() {
        assert_eq!(
            AppError::Api(ApiError::Timeout {
                timeout_secs: 10,
                endpoint: "".into()
            })
            .error_code(),
            "API_TIMEOUT"
        );
        assert_eq!(
            AppError::Api(ApiError::Http {
                status: 500,
                endpoint: "".into(),
                message: "".into()
            })
            .error_code(),
            "API_HTTP_ERROR"
        );
        assert_eq!(
            AppError::Api(ApiError::Unauthorized {
                status: 401,
                endpoint: "".into(),
                server_message: "".into()
            })
            .error_code(),
            "API_UNAUTHORIZED"
        );
        assert_eq!(
            AppError::Api(ApiError::Forbidden {
                status: 403,
                endpoint: "".into(),
                server_message: "".into()
            })
            .error_code(),
            "API_FORBIDDEN"
        );
    }

    #[test]
    fn test_error_code_auth() {
        assert_eq!(
            AppError::Auth(AuthError::NotLoggedIn).error_code(),
            "AUTH_NOT_LOGGED_IN"
        );
        assert_eq!(
            AppError::Auth(AuthError::SessionExpired).error_code(),
            "AUTH_SESSION_EXPIRED"
        );
        assert_eq!(
            AppError::Auth(AuthError::LoginFailed { message: "".into() }).error_code(),
            "AUTH_LOGIN_FAILED"
        );
    }

    #[test]
    fn test_error_code_config() {
        assert_eq!(
            AppError::Config(ConfigError::FileNotFound {
                path: "".into(),
                hint: "".into()
            })
            .error_code(),
            "CONFIG_FILE_NOT_FOUND"
        );
        assert_eq!(
            AppError::Config(ConfigError::MissingField {
                field: "".into(),
                field_type: "".into()
            })
            .error_code(),
            "CONFIG_MISSING_FIELD"
        );
        assert_eq!(
            AppError::Config(ConfigError::InvalidValue {
                field: "".into(),
                value: "".into(),
                reason: "".into()
            })
            .error_code(),
            "CONFIG_INVALID_VALUE"
        );
    }

    #[test]
    fn test_error_code_storage() {
        assert_eq!(
            AppError::Storage(StorageError::ConfigDirNotFound).error_code(),
            "STORAGE_CONFIG_DIR_NOT_FOUND"
        );
        assert_eq!(
            AppError::Storage(StorageError::ConfigParseError { message: "".into() }).error_code(),
            "STORAGE_CONFIG_PARSE"
        );
    }

    #[test]
    fn test_error_code_display() {
        assert_eq!(
            AppError::Display(DisplayError::TableFormat("".into())).error_code(),
            "DISPLAY_TABLE_FORMAT"
        );
        assert_eq!(
            AppError::Display(DisplayError::TerminalOutput("".into())).error_code(),
            "DISPLAY_TERMINAL_OUTPUT"
        );
        assert_eq!(
            AppError::Display(DisplayError::Pagination("".into())).error_code(),
            "DISPLAY_PAGINATION"
        );
    }

    #[test]
    fn test_error_code_question() {
        assert_eq!(
            AppError::Question(QuestionError::NotFound { id: 1 }).error_code(),
            "QUESTION_NOT_FOUND"
        );
        assert_eq!(
            AppError::Question(QuestionError::ExecutionFailed {
                id: 1,
                reason: "".into()
            })
            .error_code(),
            "QUESTION_EXECUTION_FAILED"
        );
        assert_eq!(
            AppError::Question(QuestionError::InvalidParameter {
                parameter: "".into()
            })
            .error_code(),
            "QUESTION_INVALID_PARAMETER"
        );
        assert_eq!(
            AppError::Question(QuestionError::ListFailed { status_code: 500 }).error_code(),
            "QUESTION_LIST_FAILED"
        );
    }

    #[test]
    fn test_error_code_service() {
        assert_eq!(
            AppError::Service(ServiceError::AuthService { message: "".into() }).error_code(),
            "SERVICE_AUTH"
        );
        assert_eq!(
            AppError::Service(ServiceError::ConfigService { message: "".into() }).error_code(),
            "SERVICE_CONFIG"
        );
        assert_eq!(
            AppError::Service(ServiceError::QuestionService { message: "".into() }).error_code(),
            "SERVICE_QUESTION"
        );
    }

    #[test]
    fn test_error_code_utils() {
        assert_eq!(
            AppError::Utils(UtilsError::Validation { message: "".into() }).error_code(),
            "UTILS_VALIDATION"
        );
        assert_eq!(
            AppError::Utils(UtilsError::DataProcessing { message: "".into() }).error_code(),
            "UTILS_DATA_PROCESSING"
        );
    }

    #[test]
    fn test_troubleshooting_hints() {
        assert!(
            AppError::Auth(AuthError::NotLoggedIn)
                .troubleshooting_hint()
                .is_some()
        );
        assert!(
            AppError::Auth(AuthError::SessionExpired)
                .troubleshooting_hint()
                .is_some()
        );
        assert!(
            AppError::Api(ApiError::Timeout {
                timeout_secs: 1,
                endpoint: "".into()
            })
            .troubleshooting_hint()
            .is_some()
        );
        assert!(
            AppError::Api(ApiError::Unauthorized {
                status: 401,
                endpoint: "".into(),
                server_message: "".into()
            })
            .troubleshooting_hint()
            .is_some()
        );
        assert!(
            AppError::Question(QuestionError::NotFound { id: 1 })
                .troubleshooting_hint()
                .is_some()
        );
        assert!(
            AppError::Display(DisplayError::Pagination("".into()))
                .troubleshooting_hint()
                .is_none()
        );
    }
}
