pub mod advanced_pagination;
pub mod display_options;
pub mod pagination;
pub mod progress;
pub mod table;

pub use advanced_pagination::AdvancedPaginationManager;
pub use display_options::{DisplayOptions, is_fullscreen_capable};
pub use pagination::{DisplayMode, PaginationConfig, PaginationManager, PaginationState};
pub use progress::{
    OperationStatus, ProgressSpinner, ProgressTracker, display_auth_result,
    display_operation_result, display_status, error_messages, show_progress_bar,
};
pub use table::{
    PaginationInfo, QuestionHeaderParams, TableDisplay, TableHeaderInfo, TableHeaderInfoBuilder,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_module_exports() {
        let _ = PaginationManager::new(10, 100, 0, DisplayMode::Paginated);
        let _ = TableDisplay::new();
        let _ = ProgressTracker::new(vec!["step1".to_string()]);
        let _ = DisplayOptions::default();
    }

    #[test]
    fn test_utils_dependency_direction() {
        use crate::utils::data::OffsetManager;
        use crate::utils::validation::validate_url;

        assert!(!OffsetManager::new(Some(10)).is_no_offset());
        assert!(OffsetManager::new(None).is_no_offset());
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("invalid-url").is_err());
    }
}
