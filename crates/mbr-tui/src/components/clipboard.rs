//! Clipboard operations for mbr-tui.
//!
//! Provides cross-platform clipboard access and format serialization
//! for copying data in various formats (JSON, CSV, TSV).

use std::fmt;

use arboard::Clipboard;
use indexmap::IndexMap;

/// Copy format options for record data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CopyFormat {
    /// JSON format (always includes keys)
    #[default]
    Json,
    /// CSV format (comma-separated)
    Csv,
    /// TSV format (tab-separated)
    Tsv,
}

impl CopyFormat {
    /// Get display label for the format.
    pub fn label(&self) -> &'static str {
        match self {
            CopyFormat::Json => "JSON",
            CopyFormat::Csv => "CSV",
            CopyFormat::Tsv => "TSV",
        }
    }

    /// Get shortcut key for the format.
    pub fn key(&self) -> char {
        match self {
            CopyFormat::Json => 'j',
            CopyFormat::Csv => 'c',
            CopyFormat::Tsv => 't',
        }
    }
}

/// Clipboard-specific error type.
#[derive(Debug, Clone)]
pub struct ClipboardError(pub String);

impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ClipboardError {}

/// Copy text to system clipboard.
pub fn copy_to_clipboard(text: &str) -> Result<(), ClipboardError> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| ClipboardError(format!("Failed to access clipboard: {}", e)))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| ClipboardError(format!("Failed to copy: {}", e)))
}

/// Format a single record (columns + values) as JSON object.
///
/// Always outputs as object format with column names as keys.
/// Keys are converted to snake_case for JSON standard compliance.
/// Column order is preserved using IndexMap.
/// Header setting does not affect JSON output.
pub fn format_record_json(columns: &[String], values: &[String]) -> String {
    let mut obj: IndexMap<String, serde_json::Value> = IndexMap::new();
    for (col, val) in columns.iter().zip(values.iter()) {
        let key = to_snake_case(col);
        obj.insert(key, serde_json::Value::String(val.clone()));
    }
    serde_json::to_string_pretty(&obj).unwrap_or_else(|_| "{}".to_string())
}

/// Convert a string to snake_case.
///
/// Handles PascalCase, camelCase, space-separated words, and acronyms.
/// Examples:
/// - "UserId" -> "user_id"
/// - "HTTPRequest" -> "http_request"
/// - "firstName" -> "first_name"
fn to_snake_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();

    for (i, c) in chars.iter().enumerate() {
        if *c == ' ' || *c == '-' || *c == '_' {
            if !result.is_empty() && !result.ends_with('_') {
                result.push('_');
            }
        } else if c.is_uppercase() {
            // Add underscore before uppercase if:
            // 1. Not at the start
            // 2. Previous char is lowercase, OR
            // 3. This is start of new word in acronym (e.g., "P" in "HTTPProxy")
            let needs_underscore = !result.is_empty()
                && !result.ends_with('_')
                && (i > 0
                    && (chars[i - 1].is_lowercase()
                        || (i + 1 < len && chars[i + 1].is_lowercase())));
            if needs_underscore {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(*c);
        }
    }
    result
}

/// Format a single record as CSV row.
///
/// When `include_header` is true, outputs header row followed by value row.
pub fn format_record_csv(columns: &[String], values: &[String], include_header: bool) -> String {
    let mut result = String::new();
    if include_header {
        result.push_str(&escape_csv_row(columns));
        result.push('\n');
    }
    result.push_str(&escape_csv_row(values));
    result
}

/// Format a single record as TSV row.
///
/// When `include_header` is true, outputs header row followed by value row.
pub fn format_record_tsv(columns: &[String], values: &[String], include_header: bool) -> String {
    let mut result = String::new();
    if include_header {
        result.push_str(&escape_tsv_row(columns));
        result.push('\n');
    }
    result.push_str(&escape_tsv_row(values));
    result
}

// === Multi-Record Format Functions ===

/// Format multiple records as JSON array.
///
/// Outputs an array of objects: [{"col1": "val1", ...}, {"col1": "val2", ...}]
/// Each record uses snake_case keys and preserves column order.
pub fn format_records_json(records: &[(Vec<String>, Vec<String>)]) -> String {
    let array: Vec<IndexMap<String, serde_json::Value>> = records
        .iter()
        .map(|(columns, values)| {
            let mut obj: IndexMap<String, serde_json::Value> = IndexMap::new();
            for (col, val) in columns.iter().zip(values.iter()) {
                let key = to_snake_case(col);
                obj.insert(key, serde_json::Value::String(val.clone()));
            }
            obj
        })
        .collect();
    serde_json::to_string_pretty(&array).unwrap_or_else(|_| "[]".to_string())
}

/// Format multiple records as CSV.
///
/// When `include_header` is true, outputs header row followed by data rows.
/// Uses column names from the first record.
pub fn format_records_csv(records: &[(Vec<String>, Vec<String>)], include_header: bool) -> String {
    if records.is_empty() {
        return String::new();
    }

    let mut result = String::new();

    // Add header from first record's columns
    if include_header {
        result.push_str(&escape_csv_row(&records[0].0));
        result.push('\n');
    }

    // Add data rows
    for (_, values) in records {
        result.push_str(&escape_csv_row(values));
        result.push('\n');
    }

    // Remove trailing newline
    result.pop();
    result
}

/// Format multiple records as TSV.
///
/// When `include_header` is true, outputs header row followed by data rows.
/// Uses column names from the first record.
pub fn format_records_tsv(records: &[(Vec<String>, Vec<String>)], include_header: bool) -> String {
    if records.is_empty() {
        return String::new();
    }

    let mut result = String::new();

    // Add header from first record's columns
    if include_header {
        result.push_str(&escape_tsv_row(&records[0].0));
        result.push('\n');
    }

    // Add data rows
    for (_, values) in records {
        result.push_str(&escape_tsv_row(values));
        result.push('\n');
    }

    // Remove trailing newline
    result.pop();
    result
}

/// Escape and join values as CSV row.
///
/// Handles values containing commas, quotes, or newlines.
fn escape_csv_row(values: &[String]) -> String {
    values
        .iter()
        .map(|v| {
            if v.contains(',') || v.contains('"') || v.contains('\n') || v.contains('\r') {
                format!("\"{}\"", v.replace('"', "\"\""))
            } else {
                v.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Escape and join values as TSV row.
///
/// Replaces tabs and newlines with spaces for clean output.
fn escape_tsv_row(values: &[String]) -> String {
    values
        .iter()
        .map(|v| v.replace(['\t', '\n', '\r'], " "))
        .collect::<Vec<_>>()
        .join("\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_record_json_preserves_order() {
        let columns = vec![
            "ZField".to_string(),
            "AField".to_string(),
            "MField".to_string(),
        ];
        let values = vec![
            "z_val".to_string(),
            "a_val".to_string(),
            "m_val".to_string(),
        ];
        let result = format_record_json(&columns, &values);
        // Verify order is preserved (Z before A before M, not alphabetical)
        let z_pos = result.find("z_field").unwrap();
        let a_pos = result.find("a_field").unwrap();
        let m_pos = result.find("m_field").unwrap();
        assert!(z_pos < a_pos, "ZField should come before AField");
        assert!(a_pos < m_pos, "AField should come before MField");
    }

    #[test]
    fn test_format_record_json_snake_case() {
        let columns = vec![
            "UserId".to_string(),
            "FirstName".to_string(),
            "lastLoginDate".to_string(),
        ];
        let values = vec![
            "1".to_string(),
            "John".to_string(),
            "2024-01-01".to_string(),
        ];
        let result = format_record_json(&columns, &values);
        assert!(result.contains("\"user_id\": \"1\""));
        assert!(result.contains("\"first_name\": \"John\""));
        assert!(result.contains("\"last_login_date\": \"2024-01-01\""));
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("UserId"), "user_id");
        assert_eq!(to_snake_case("firstName"), "first_name");
        assert_eq!(to_snake_case("HTTPRequest"), "http_request");
        assert_eq!(to_snake_case("HTTPProxy"), "http_proxy");
        assert_eq!(to_snake_case("already_snake"), "already_snake");
        assert_eq!(to_snake_case("With Spaces"), "with_spaces");
        assert_eq!(to_snake_case("kebab-case"), "kebab_case");
        assert_eq!(to_snake_case("XMLParser"), "xml_parser");
        assert_eq!(to_snake_case("getHTTPResponse"), "get_http_response");
    }

    #[test]
    fn test_format_record_csv_with_header() {
        let columns = vec!["id".to_string(), "name".to_string()];
        let values = vec!["123".to_string(), "Test".to_string()];
        let result = format_record_csv(&columns, &values, true);
        assert_eq!(result, "id,name\n123,Test");
    }

    #[test]
    fn test_format_record_csv_without_header() {
        let columns = vec!["id".to_string(), "name".to_string()];
        let values = vec!["123".to_string(), "Test".to_string()];
        let result = format_record_csv(&columns, &values, false);
        assert_eq!(result, "123,Test");
    }

    #[test]
    fn test_format_record_csv_escape() {
        let columns = vec!["name".to_string()];
        let values = vec!["Hello, \"World\"".to_string()];
        let result = format_record_csv(&columns, &values, false);
        assert_eq!(result, "\"Hello, \"\"World\"\"\"");
    }

    #[test]
    fn test_format_record_tsv_with_header() {
        let columns = vec!["id".to_string(), "name".to_string()];
        let values = vec!["123".to_string(), "Test".to_string()];
        let result = format_record_tsv(&columns, &values, true);
        assert_eq!(result, "id\tname\n123\tTest");
    }

    #[test]
    fn test_format_record_tsv_escape() {
        let columns = vec!["desc".to_string()];
        let values = vec!["Line1\tLine2\nLine3".to_string()];
        let result = format_record_tsv(&columns, &values, false);
        assert_eq!(result, "Line1 Line2 Line3");
    }
}
