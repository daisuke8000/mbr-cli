use crate::error::AppError;

/// Struct to manage display options
#[derive(Debug, Clone)]
pub struct DisplayOptions {
    /// Full mode (display all data at once)
    pub full: bool,
    /// Start position (offset)
    pub offset: Option<usize>,
    /// Specification of columns to display
    pub columns: Option<String>,
    /// Page size
    pub page_size: usize,
    /// Disable fullscreen mode
    pub no_fullscreen: bool,
    /// Question ID (for display)
    pub question_id: Option<u32>,
    /// Memory limit (MB)
    pub max_memory_mb: usize,
    /// Disable the use of colors
    pub no_color: bool,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            full: false,
            offset: None,
            columns: None,
            page_size: 20,
            no_fullscreen: false,
            question_id: None,
            max_memory_mb: 500, // Default 500MB limit
            no_color: false,
        }
    }
}

impl DisplayOptions {
    /// Create a new DisplayOptions instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Set full mode
    pub fn with_full_mode(mut self, full: bool) -> Self {
        self.full = full;
        self
    }

    /// Set offset
    pub fn with_offset(mut self, offset: Option<usize>) -> Self {
        self.offset = offset;
        self
    }

    /// Set column specification
    pub fn with_columns(mut self, columns: Option<String>) -> Self {
        self.columns = columns;
        self
    }

    /// Set page size
    pub fn with_page_size(mut self, page_size: usize) -> Self {
        self.page_size = page_size;
        self
    }

    /// Set fullscreen disable
    pub fn with_no_fullscreen(mut self, no_fullscreen: bool) -> Self {
        self.no_fullscreen = no_fullscreen;
        self
    }

    /// Set question ID
    pub fn with_question_id(mut self, question_id: Option<u32>) -> Self {
        self.question_id = question_id;
        self
    }

    /// Set a memory limit
    pub fn with_max_memory(mut self, max_memory_mb: usize) -> Self {
        self.max_memory_mb = max_memory_mb;
        self
    }

    /// Set color disable
    pub fn with_no_color(mut self, no_color: bool) -> Self {
        self.no_color = no_color;
        self
    }

    /// Set options from environment variables
    pub fn from_env() -> Self {
        let mut options = Self::default();

        // Read environment variables
        if std::env::var("NO_COLOR").is_ok() {
            options.no_color = true;
        }

        if let Ok(page_size_str) = std::env::var("MBR_PAGE_SIZE") {
            if let Ok(page_size) = page_size_str.parse::<usize>() {
                options.page_size = page_size;
            }
        }

        if std::env::var("MBR_NO_FULLSCREEN").is_ok() {
            options.no_fullscreen = true;
        }

        if let Ok(memory_str) = std::env::var("MBR_MAX_MEMORY") {
            if let Ok(memory_mb) = memory_str.parse::<usize>() {
                options.max_memory_mb = memory_mb;
            }
        }

        options
    }

    /// Validate option validity
    pub fn validate(&self) -> Result<(), AppError> {
        if self.page_size == 0 {
            return Err(AppError::Display(crate::error::DisplayError::Pagination(
                "Page size must be greater than 0".to_string(),
            )));
        }

        if self.max_memory_mb == 0 {
            return Err(AppError::Display(
                crate::error::DisplayError::TerminalOutput(
                    "Memory limit must be greater than 0".to_string(),
                ),
            ));
        }

        if let Some(offset) = self.offset {
            if offset == usize::MAX {
                return Err(AppError::Display(crate::error::DisplayError::Pagination(
                    "Invalid offset value".to_string(),
                )));
            }
        }

        Ok(())
    }

    /// Determine display mode
    pub fn determine_display_mode(&self, total_items: usize) -> DisplayMode {
        if self.full {
            return DisplayMode::Full;
        }

        if total_items <= self.page_size {
            return DisplayMode::Simple;
        }

        if self.no_fullscreen || !is_fullscreen_capable() {
            return DisplayMode::Paginated;
        }

        DisplayMode::Interactive
    }
}

/// Types of display modes
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayMode {
    /// Simple display (for small datasets)
    Simple,
    /// Display all data at once
    Full,
    /// Paginated display
    Paginated,
    /// Interactive display
    Interactive,
}

/// Check the availability of fullscreen functionality
pub fn is_fullscreen_capable() -> bool {
    // Check if TTY
    if !atty::is(atty::Stream::Stdout) {
        return false;
    }

    // Check CI environment
    if is_ci_environment() {
        return false;
    }

    // Check SSH environment
    if is_ssh_environment() {
        return false;
    }

    // Check WSL environment
    if is_wsl_environment() {
        return false;
    }

    // Check terminal support
    is_terminal_supported()
}

/// Determine if CI environment
fn is_ci_environment() -> bool {
    std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("GITLAB_CI").is_ok()
        || std::env::var("JENKINS_URL").is_ok()
        || std::env::var("BUILDKITE").is_ok()
}

/// Determine if SSH environment
fn is_ssh_environment() -> bool {
    std::env::var("SSH_CONNECTION").is_ok() || std::env::var("SSH_CLIENT").is_ok()
}

/// Determine if WSL environment
fn is_wsl_environment() -> bool {
    std::env::var("WSL_DISTRO_NAME").is_ok()
        || std::env::var("WSLENV").is_ok()
        || if let Ok(version) = std::fs::read_to_string("/proc/version") {
            version.contains("Microsoft") || version.contains("WSL")
        } else {
            false
        }
}

/// Check terminal support
fn is_terminal_supported() -> bool {
    // Check TERM environment variable
    match std::env::var("TERM") {
        Ok(term) => !term.is_empty() && !term.starts_with("dumb"),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_options_creation() {
        let options = DisplayOptions::new();
        assert!(!options.full);
        assert_eq!(options.page_size, 20);
        assert!(!options.no_fullscreen);
        assert_eq!(options.max_memory_mb, 500);
    }

    #[test]
    fn test_display_options_builder() {
        let options = DisplayOptions::new()
            .with_full_mode(true)
            .with_page_size(50)
            .with_offset(Some(100))
            .with_question_id(Some(123));

        assert!(options.full);
        assert_eq!(options.page_size, 50);
        assert_eq!(options.offset, Some(100));
        assert_eq!(options.question_id, Some(123));
    }

    #[test]
    fn test_display_options_validation() {
        let valid_options = DisplayOptions::new();
        assert!(valid_options.validate().is_ok());

        let invalid_options = DisplayOptions::new().with_page_size(0);
        assert!(invalid_options.validate().is_err());

        let invalid_memory = DisplayOptions::new().with_max_memory(0);
        assert!(invalid_memory.validate().is_err());
    }

    #[test]
    fn test_display_mode_determination() {
        let options = DisplayOptions::new();

        // Small dataset
        assert_eq!(options.determine_display_mode(10), DisplayMode::Simple);

        // Full mode specified
        let full_options = DisplayOptions::new().with_full_mode(true);
        assert_eq!(full_options.determine_display_mode(100), DisplayMode::Full);

        // Fullscreen disabled
        let no_fs_options = DisplayOptions::new().with_no_fullscreen(true);
        let mode = no_fs_options.determine_display_mode(100);
        assert!(mode == DisplayMode::Paginated || mode == DisplayMode::Interactive);
    }

    #[test]
    fn test_environment_detection() {
        // These functions are environment-dependent, so only basic calls are tested
        let _ = is_ci_environment();
        let _ = is_ssh_environment();
        let _ = is_wsl_environment();
        let _ = is_terminal_supported();
        let _ = is_fullscreen_capable();
    }
}
