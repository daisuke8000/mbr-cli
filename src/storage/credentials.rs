use super::Result;
use serde::{Deserialize, Serialize};
use std::env;

#[cfg(not(test))]
use keyring::Entry;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Credentials {
    email: Option<String>,
    password: Option<String>,
    session_token: Option<String>,
    pub profile_name: String,
}

pub enum AuthMode {
    APIKey,
    Session,
}

impl Credentials {
    pub fn new(profile_name: String) -> Self {
        Self {
            email: None,
            password: None,
            session_token: None,
            profile_name,
        }
    }

    pub fn load(profile_name: &str) -> Result<Self> {
        let mut credentials = Self::new(profile_name.to_string());
        credentials.password = credentials.load_credentials("password")?;
        credentials.session_token = credentials.load_credentials("session")?;
        Ok(credentials)
    }

    #[cfg(not(test))]
    fn load_credentials(&self, key_type: &str) -> Result<Option<String>> {
        let entry = Entry::new("mbr-cli", &format!("{}-{}", key_type, self.profile_name))
            .map_err(|e| crate::error::StorageError::KeyringError(e.to_string()))?;

        match entry.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(crate::error::StorageError::KeyringError(e.to_string())),
        }
    }

    #[cfg(test)]
    fn load_credentials(&self, key_type: &str) -> Result<Option<String>> {
        println!(
            "MOCK: Loading {} for profile {}",
            key_type, self.profile_name
        );
        Ok(None) // Mock implementation for tests
    }

    pub fn save(&self) -> Result<()> {
        self.save_credentials("password", &self.password)?;
        self.save_credentials("session", &self.session_token)?;
        Ok(())
    }

    // use login
    pub fn save_session_for_profile(profile_name: &str, token: &str) -> Result<()> {
        let mut credentials = Self::new(profile_name.to_string());
        credentials.session_token = Some(token.to_string());
        credentials.save_credentials("session", &credentials.session_token)?;
        Ok(())
    }

    // use logout
    pub fn clear_session_for_profile(profile_name: &str) -> Result<()> {
        let credentials = Self::new(profile_name.to_string());
        credentials.save_credentials("session", &None)?;
        Ok(())
    }

    #[cfg(not(test))]
    fn save_credentials(&self, key_type: &str, value: &Option<String>) -> Result<()> {
        if let Some(v) = value {
            let entry = Entry::new("mbr-cli", &format!("{}-{}", key_type, self.profile_name))
                .map_err(|e| crate::error::StorageError::KeyringError(e.to_string()))?;
            entry
                .set_password(v)
                .map_err(|e| crate::error::StorageError::KeyringError(e.to_string()))?;
        }

        Ok(())
    }

    #[cfg(test)]
    fn save_credentials(&self, key_type: &str, value: &Option<String>) -> Result<()> {
        if let Some(v) = value {
            println!(
                "MOCK: Saving {} = '{}' for profile {}",
                key_type, v, self.profile_name
            );
        } else {
            println!(
                "MOCK: Skipping save for {} (None value) for profile {}",
                key_type, self.profile_name
            );
        }
        Ok(()) // Mock implementation for tests
    }

    #[cfg(not(test))]
    fn has_api_key() -> bool {
        env::var("MBR_API_KEY").is_ok()
    }

    #[cfg(test)]
    fn has_api_key() -> bool {
        env::var("TEST_MBR_API_KEY").is_ok()
    }

    pub fn get_auth_mode(&self) -> AuthMode {
        if Self::has_api_key() {
            AuthMode::APIKey
        } else {
            AuthMode::Session
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_credentials_mock() {
        let mut creds = Credentials::new("test-profile".to_string());
        creds.password = Some("test-password".to_string());
        creds.session_token = Some("test-session".to_string());

        let result = creds.save();
        assert!(result.is_ok(), "Save should succeed in test environment");
    }

    #[test]
    fn test_load_credentials_mock() {
        let loaded = Credentials::load("test-profile");
        assert!(loaded.is_ok(), "Load should succeed in test environment");

        let creds = loaded.expect("Loaded credentials should not be None");
        assert_eq!(creds.profile_name, "test-profile");
        assert!(creds.password.is_none(), "Password should be None in mock");
        assert!(
            creds.session_token.is_none(),
            "Session token should be None in mock"
        );
    }

    #[test]
    fn test_get_auth_mode_with_api_key() {
        // 環境変数の初期状態を保存
        let original_key = env::var("TEST_MBR_API_KEY").ok();

        unsafe {
            env::set_var("TEST_MBR_API_KEY", "test_api_key");
        }
        let creds = Credentials::new("test".to_string());
        assert!(matches!(creds.get_auth_mode(), AuthMode::APIKey));

        // 環境変数を元の状態に復元
        unsafe {
            match original_key {
                Some(value) => env::set_var("TEST_MBR_API_KEY", value),
                None => env::remove_var("TEST_MBR_API_KEY"),
            }
        }
    }

    #[test]
    fn test_get_auth_mode_without_api_key() {
        // 環境変数の初期状態を保存
        let original_key = env::var("TEST_MBR_API_KEY").ok();

        unsafe {
            env::remove_var("TEST_MBR_API_KEY");
        }
        let creds = Credentials::new("test".to_string());
        assert!(matches!(creds.get_auth_mode(), AuthMode::Session));

        // 環境変数を元の状態に復元
        unsafe {
            match original_key {
                Some(value) => env::set_var("TEST_MBR_API_KEY", value),
                None => env::remove_var("TEST_MBR_API_KEY"),
            }
        }
    }
}
