//! Application actions for event-driven architecture.
//!
//! Implements Flux-like unidirectional data flow:
//! Component → AppAction → App State → Component Re-render

// Allow unused variants as they are designed for Phase 4+ implementation
#![allow(dead_code)]

/// Application-level actions for component-to-app communication.
///
/// This enum enables clean separation between UI components and application logic.
/// Components emit actions, the App processes them and updates state.
#[derive(Debug, Clone, PartialEq)]
pub enum AppAction {
    /// Quit the application
    Quit,

    /// Change focus to the next panel
    NextPanel,

    /// Change focus to the previous panel
    PreviousPanel,

    /// Navigate to a specific content view
    Navigate(ContentTarget),

    /// Load data from API
    LoadData(DataRequest),

    /// Show an error message
    ShowError(String),

    /// Clear current error message
    ClearError,

    /// Update status message
    SetStatus(String),

    /// Clear status message
    ClearStatus,
}

/// Target content views for navigation
#[derive(Debug, Clone, PartialEq)]
pub enum ContentTarget {
    /// Welcome/home screen
    Welcome,
    /// Questions list view
    Questions,
    /// Collections list view
    Collections,
    /// Databases list view
    Databases,
    /// Settings view
    Settings,
}

/// Data loading requests
#[derive(Debug, Clone, PartialEq)]
pub enum DataRequest {
    /// Load questions list
    Questions,
    /// Load a specific question's details
    QuestionDetails(u32),
    /// Refresh current data
    Refresh,
}

impl From<usize> for ContentTarget {
    fn from(index: usize) -> Self {
        match index {
            0 => ContentTarget::Questions,
            1 => ContentTarget::Collections,
            2 => ContentTarget::Databases,
            3 => ContentTarget::Settings,
            _ => ContentTarget::Welcome,
        }
    }
}
