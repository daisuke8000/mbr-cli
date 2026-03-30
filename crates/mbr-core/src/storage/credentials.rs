//! Session credential management
//!
//! This module handles session-based authentication with Metabase.
//! Sessions are stored in ~/.config/mbr-cli/session.json.
//! Username/password can be provided via MBR_USERNAME/MBR_PASSWORD environment variables.

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

/// Stored session data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Session {
    pub session_token: String,
    pub url: String,
    pub username: String,
    pub created_at: String,
}

/// Format current time as ISO 8601 string (e.g., "2026-03-30T12:00:00Z").
pub fn now_iso8601() -> String {
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;
    let (year, month, day) = epoch_days_to_date(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn epoch_days_to_date(days: u64) -> (u64, u64, u64) {
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Get the session file path: ~/.config/mbr-cli/session.json
fn session_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".config").join("mbr-cli").join("session.json"))
}

/// Save a session to disk with restricted file permissions (0600 on Unix).
pub fn save_session(session: &Session) -> Result<(), String> {
    let path = session_file_path().ok_or("Could not determine home directory")?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let json = serde_json::to_string_pretty(session)
        .map_err(|e| format!("Failed to serialize session: {}", e))?;

    fs::write(&path, &json).map_err(|e| format!("Failed to write session file: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&path, perms)
            .map_err(|e| format!("Failed to set session file permissions: {}", e))?;
    }

    Ok(())
}

/// Load a session from disk. Returns None if the file does not exist.
pub fn load_session() -> Option<Session> {
    let path = session_file_path()?;
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Delete the stored session file.
pub fn delete_session() -> Result<(), String> {
    let path = session_file_path().ok_or("Could not determine home directory")?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Failed to delete session file: {}", e))?;
    }
    Ok(())
}

/// Get login credentials from environment variables.
/// Returns (username, password) if both MBR_USERNAME and MBR_PASSWORD are set and non-empty.
pub fn get_credentials() -> Option<(String, String)> {
    let username = env::var("MBR_USERNAME").ok().filter(|s| !s.is_empty())?;
    let password = env::var("MBR_PASSWORD").ok().filter(|s| !s.is_empty())?;
    Some((username, password))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_iso8601_format() {
        let ts = now_iso8601();
        assert!(ts.ends_with('Z'));
        assert_eq!(ts.len(), 20);
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[10..11], "T");
    }

    #[test]
    fn test_session_serialization() {
        let session = Session {
            session_token: "test-token-123".to_string(),
            url: "http://localhost:3000".to_string(),
            username: "test@example.com".to_string(),
            created_at: "2026-03-30T12:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.session_token, "test-token-123");
        assert_eq!(deserialized.url, "http://localhost:3000");
        assert_eq!(deserialized.username, "test@example.com");
    }

    #[test]
    fn test_get_credentials_when_both_set() {
        let orig_user = env::var("MBR_USERNAME").ok();
        let orig_pass = env::var("MBR_PASSWORD").ok();

        unsafe {
            env::set_var("MBR_USERNAME", "user@test.com");
            env::set_var("MBR_PASSWORD", "secret123");
        }

        let creds = get_credentials();
        assert!(creds.is_some());
        let (user, pass) = creds.unwrap();
        assert_eq!(user, "user@test.com");
        assert_eq!(pass, "secret123");

        unsafe {
            match orig_user {
                Some(v) => env::set_var("MBR_USERNAME", v),
                None => env::remove_var("MBR_USERNAME"),
            }
            match orig_pass {
                Some(v) => env::set_var("MBR_PASSWORD", v),
                None => env::remove_var("MBR_PASSWORD"),
            }
        }
    }

    #[test]
    fn test_get_credentials_when_missing() {
        let orig_user = env::var("MBR_USERNAME").ok();
        let orig_pass = env::var("MBR_PASSWORD").ok();

        unsafe {
            env::remove_var("MBR_USERNAME");
            env::remove_var("MBR_PASSWORD");
        }

        assert!(get_credentials().is_none());

        unsafe {
            match orig_user {
                Some(v) => env::set_var("MBR_USERNAME", v),
                None => env::remove_var("MBR_USERNAME"),
            }
            match orig_pass {
                Some(v) => env::set_var("MBR_PASSWORD", v),
                None => env::remove_var("MBR_PASSWORD"),
            }
        }
    }

    #[test]
    fn test_get_credentials_when_empty() {
        let orig_user = env::var("MBR_USERNAME").ok();
        let orig_pass = env::var("MBR_PASSWORD").ok();

        unsafe {
            env::set_var("MBR_USERNAME", "");
            env::set_var("MBR_PASSWORD", "pass");
        }
        assert!(get_credentials().is_none());

        unsafe {
            match orig_user {
                Some(v) => env::set_var("MBR_USERNAME", v),
                None => env::remove_var("MBR_USERNAME"),
            }
            match orig_pass {
                Some(v) => env::set_var("MBR_PASSWORD", v),
                None => env::remove_var("MBR_PASSWORD"),
            }
        }
    }
}
