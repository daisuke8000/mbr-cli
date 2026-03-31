use clap::ValueEnum;
use mbr_core::error::AppError;
use serde::Serialize;

/// Output format for command results
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Render as a human-readable table
    #[default]
    Table,
    /// Render as JSON
    Json,
    /// Render as CSV
    Csv,
}

/// Resolve the effective output format.
/// The global `--json` flag overrides the per-command `--format` option.
pub fn resolve_format(json_flag: bool, command_format: OutputFormat) -> OutputFormat {
    if json_flag {
        OutputFormat::Json
    } else {
        command_format
    }
}

// ── Structured output types for JSON mode ──────────────────────────────

#[derive(Serialize)]
pub struct StatusOutput {
    pub url: Option<String>,
    pub session: Option<SessionInfo>,
}

#[derive(Serialize)]
pub struct SessionInfo {
    pub username: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct LoginOutput {
    pub success: bool,
    pub username: String,
    pub url: String,
}

#[derive(Serialize)]
pub struct LogoutOutput {
    pub success: bool,
}

#[derive(Serialize)]
pub struct ConfigSetOutput {
    pub success: bool,
    pub url: String,
}

#[derive(Serialize)]
pub struct ConfigValidateOutput {
    pub valid: bool,
    pub user: Option<ValidateUserInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct ValidateUserInfo {
    pub id: u32,
    pub email: String,
    pub name: Option<String>,
    pub is_superuser: Option<bool>,
}

// ── Error output types ─────────────────────────────────────────────────

#[derive(Serialize)]
pub struct JsonErrorOutput {
    pub error: JsonErrorDetail,
}

#[derive(Serialize)]
pub struct JsonErrorDetail {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Pretty-print any serializable value as JSON to stdout.
pub fn print_json<T: Serialize>(data: &T) {
    match serde_json::to_string_pretty(data) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error serializing JSON: {}", e),
    }
}

/// Print a structured JSON error to stdout (so callers piping JSON get it).
pub fn print_json_error(error: &AppError) {
    let output = JsonErrorOutput {
        error: JsonErrorDetail {
            code: error.error_code().to_string(),
            message: error.display_friendly(),
            hint: error.troubleshooting_hint(),
        },
    };
    print_json(&output);
}

/// Re-export CSV escaping from mbr-core for use in command handlers.
pub use mbr_core::utils::text::escape_csv_field;

/// Map an `AppError` variant to a process exit code.
pub fn exit_code_for(error: &AppError) -> i32 {
    match error {
        AppError::Cli(_) => 1,
        AppError::Api(_) => 2,
        AppError::Auth(_) => 3,
        AppError::Config(_) => 4,
        AppError::Storage(_) => 4,
        AppError::Display(_) => 1,
        AppError::Question(_) => 2,
        AppError::Service(_) => 1,
        AppError::Utils(_) => 1,
    }
}
