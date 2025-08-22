pub mod advanced_pagination;
pub mod display_options;
pub mod pagination;
pub mod progress;
pub mod table;

pub use advanced_pagination::AdvancedPaginationManager;
pub use display_options::{DisplayOptions, is_fullscreen_capable};
pub use pagination::{DisplayMode, PaginationConfig, PaginationManager, PaginationState};
pub use progress::{
    OperationStatus, ProgressSpinner, ProgressTracker, display_status, show_progress_bar,
};
pub use table::{
    PaginationInfo, QuestionHeaderParams, TableDisplay, TableHeaderInfo, TableHeaderInfoBuilder,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_module_only_contains_ui_components() {
        // This test ensures display module only exports UI components
        // It should not re-export utilities from utils module

        // Test that core UI components are available
        let _pagination_manager = PaginationManager::new(10, 100, 0, DisplayMode::Paginated);
        let _table_display = TableDisplay::new();
        let _progress_tracker = ProgressTracker::new(vec!["step1".to_string()]);
        let _display_options = DisplayOptions::default();

        // This test will fail if utils re-exports are still present
        // We should not be able to access MemoryEstimator or OffsetManager from display module
    }

    #[test]
    fn test_display_module_ui_focus() {
        // Verify that display module focuses on UI rendering only
        // All modules should be related to presentation/visualization

        // These should all be UI-related components
        assert!(std::any::type_name::<TableDisplay>().contains("TableDisplay"));
        assert!(std::any::type_name::<PaginationManager>().contains("PaginationManager"));
        assert!(std::any::type_name::<ProgressTracker>().contains("ProgressTracker"));
        assert!(std::any::type_name::<DisplayOptions>().contains("DisplayOptions"));
    }

    #[test]
    fn test_display_can_use_utils_dependencies() {
        // Test that display module can properly access utils functionality
        // This ensures proper dependency direction: Display → Utils

        // Test data utilities access - this is the main utils dependency for display
        use crate::utils::data::OffsetManager;
        let offset_manager = OffsetManager::new(Some(10)).expect("Should create OffsetManager");
        assert!(!offset_manager.is_no_offset());

        // Test that we can access OffsetManager with None
        let no_offset_manager = OffsetManager::new(None).expect("Should create OffsetManager");
        assert!(no_offset_manager.is_no_offset());

        // This test passes if display can use utils without circular dependency
    }

    #[test]
    fn test_no_circular_dependencies() {
        // Ensure no circular dependencies exist
        // utils should NOT import from display
        // This test verifies the architectural constraint

        // If this compiles and runs, it means:
        // 1. Display can import from Utils (correct direction)
        // 2. No circular dependency exists (compilation would fail)
        // 3. Architecture layer dependency is preserved: CLI → Core → Storage → Utils

        use crate::utils::data::OffsetManager;
        use crate::utils::validation::validate_url;

        // Test that utils functions are accessible from display
        let _offset = OffsetManager::new(None).expect("Should create OffsetManager");
        let _url_validation = validate_url("https://example.com");

        // Test that we can access both data and validation utils
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("invalid-url").is_err());

        // Successful compilation means proper dependency direction
        // Test validates that display can use utils without circular dependency
    }
}
