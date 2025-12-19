//! UI Components for mbr-tui.
//!
//! This module provides reusable UI components with a common interface.

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

mod content;
mod navigation;
mod status_bar;

pub use content::{ContentPanel, ContentView};
pub use navigation::NavigationPanel;
pub use status_bar::StatusBar;

/// Active panel for focus management.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivePanel {
    #[default]
    Navigation,
    Content,
}

impl ActivePanel {
    /// Cycle to the next panel (Tab key).
    pub fn next(self) -> Self {
        match self {
            ActivePanel::Navigation => ActivePanel::Content,
            ActivePanel::Content => ActivePanel::Navigation,
        }
    }

    /// Cycle to the previous panel (Shift+Tab).
    pub fn previous(self) -> Self {
        match self {
            ActivePanel::Navigation => ActivePanel::Content,
            ActivePanel::Content => ActivePanel::Navigation,
        }
    }
}

/// Common trait for all UI components.
pub trait Component {
    /// Draw the component within the given area.
    fn draw(&self, frame: &mut Frame, area: Rect, focused: bool);

    /// Handle keyboard input. Returns true if the event was consumed.
    fn handle_key(&mut self, key: KeyEvent) -> bool;

    /// Get the component's title for the border.
    fn title(&self) -> &str;
}

/// Scroll state for components with scrollable content.
#[derive(Debug, Default, Clone)]
pub struct ScrollState {
    /// Current scroll offset (top visible item index).
    pub offset: usize,
    /// Total number of items.
    pub total: usize,
    /// Number of visible items.
    pub visible: usize,
}

impl ScrollState {
    /// Create a new scroll state.
    pub fn new(total: usize, visible: usize) -> Self {
        Self {
            offset: 0,
            total,
            visible,
        }
    }

    /// Scroll up by one item.
    pub fn scroll_up(&mut self) {
        self.offset = self.offset.saturating_sub(1);
    }

    /// Scroll down by one item.
    pub fn scroll_down(&mut self) {
        if self.offset + self.visible < self.total {
            self.offset += 1;
        }
    }

    /// Scroll to a specific item index.
    pub fn scroll_to(&mut self, index: usize) {
        if index < self.total {
            // Ensure the item is visible
            if index < self.offset {
                self.offset = index;
            } else if index >= self.offset + self.visible {
                self.offset = index.saturating_sub(self.visible.saturating_sub(1));
            }
        }
    }
}
