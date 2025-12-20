//! UI Components for mbr-tui.
//!
//! This module provides reusable UI components with a common interface.

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

pub mod clipboard;
mod content;
mod copy_menu;
mod help_overlay;
mod record_detail;
pub mod state_renderer;
mod status_bar;
pub mod styles;

pub use content::{ContentPanel, ContentView, InputMode, QueryResultData};
pub use copy_menu::CopyMenu;
pub use help_overlay::HelpOverlay;
pub use record_detail::RecordDetailOverlay;
pub use status_bar::StatusBar;

/// Active tab for navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveTab {
    #[default]
    Questions,
    Collections,
    Databases,
}

impl ActiveTab {
    /// Get the index of this tab.
    pub fn index(self) -> usize {
        match self {
            ActiveTab::Questions => 0,
            ActiveTab::Collections => 1,
            ActiveTab::Databases => 2,
        }
    }

    /// Create from index.
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => ActiveTab::Questions,
            1 => ActiveTab::Collections,
            2 => ActiveTab::Databases,
            _ => ActiveTab::Questions,
        }
    }

    /// Get the next tab (cycling).
    pub fn next(self) -> Self {
        Self::from_index((self.index() + 1) % 3)
    }

    /// Get the previous tab (cycling).
    pub fn previous(self) -> Self {
        Self::from_index((self.index() + 2) % 3)
    }

    /// Get display label with icon.
    pub fn label(self) -> &'static str {
        match self {
            ActiveTab::Questions => "📋 Questions",
            ActiveTab::Collections => "📁 Collections",
            ActiveTab::Databases => "🗄️ Databases",
        }
    }
}

/// Common trait for all UI components.
pub trait Component {
    /// Draw the component within the given area.
    /// Takes `&mut self` to support stateful widgets like TableState.
    fn draw(&mut self, frame: &mut Frame, area: Rect, focused: bool);

    /// Handle keyboard input. Returns true if the event was consumed.
    #[allow(dead_code)]
    fn handle_key(&mut self, key: KeyEvent) -> bool;
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
}
