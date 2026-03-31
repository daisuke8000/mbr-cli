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

/// Escape a field value for RFC 4180 compliant CSV output.
/// Wraps in double-quotes if the value contains comma, double-quote, or newline.
/// Internal double-quotes are escaped by doubling them.
pub fn escape_csv_field(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_csv_field_plain() {
        assert_eq!(escape_csv_field("hello"), "hello");
    }

    #[test]
    fn test_escape_csv_field_with_comma() {
        assert_eq!(escape_csv_field("Sales, Q4"), "\"Sales, Q4\"");
    }

    #[test]
    fn test_escape_csv_field_with_quotes() {
        assert_eq!(escape_csv_field("say \"hello\""), "\"say \"\"hello\"\"\"");
    }

    #[test]
    fn test_escape_csv_field_with_newline() {
        assert_eq!(escape_csv_field("line1\nline2"), "\"line1\nline2\"");
    }

    #[test]
    fn test_escape_csv_field_empty() {
        assert_eq!(escape_csv_field(""), "");
    }
}
