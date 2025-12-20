//! Shared style definitions for TUI components.
//!
//! This module provides consistent styling across all TUI views,
//! eliminating duplication and ensuring visual consistency.
//!
//! Some styles are reserved for future View refactoring (PR 5, 6).

#![allow(dead_code)]

use ratatui::style::{Color, Modifier, Style};

// === Border Styles ===

/// Border style for focused components.
pub const BORDER_FOCUSED: Style = Style::new().fg(Color::Cyan);

/// Border style for unfocused components.
pub const BORDER_UNFOCUSED: Style = Style::new().fg(Color::DarkGray);

/// Get border style based on focus state.
#[inline]
pub fn border_style(focused: bool) -> Style {
    if focused {
        BORDER_FOCUSED
    } else {
        BORDER_UNFOCUSED
    }
}

// === Table Styles ===

/// Style for table header text.
pub fn header_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

/// Style for table row when selected/highlighted.
pub fn row_highlight_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

/// Default highlight symbol for table selection.
pub const HIGHLIGHT_SYMBOL: &str = "► ";

// === Text Styles ===

/// Style for dimmed/hint text.
pub const TEXT_DIM: Style = Style::new().fg(Color::DarkGray);

/// Style for warning/loading text.
pub const TEXT_WARNING: Style = Style::new().fg(Color::Yellow);

/// Style for error text.
pub const TEXT_ERROR: Style = Style::new().fg(Color::Red);

/// Style for success text.
pub const TEXT_SUCCESS: Style = Style::new().fg(Color::Green);

/// Style for bold white text.
pub fn text_bold_white() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

// === Modal Styles ===

/// Style for modal titles.
pub fn modal_title_style() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

/// Style for selected items in lists.
pub fn selected_style() -> Style {
    Style::default()
        .bg(Color::Blue)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

/// Style for selected index numbers.
pub fn selected_index_style() -> Style {
    Style::default().fg(Color::Cyan)
}

// === Input Styles ===

/// Style for cursor indicator.
pub fn cursor_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::SLOW_BLINK)
}

/// Style for input text.
pub fn input_text_style() -> Style {
    Style::default().fg(Color::White)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_border_style_focused() {
        let style = border_style(true);
        assert_eq!(style.fg, Some(Color::Cyan));
    }

    #[test]
    fn test_border_style_unfocused() {
        let style = border_style(false);
        assert_eq!(style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn test_header_style() {
        let style = header_style();
        assert_eq!(style.fg, Some(Color::Yellow));
    }

    #[test]
    fn test_row_highlight_style() {
        let style = row_highlight_style();
        assert_eq!(style.bg, Some(Color::Cyan));
        assert_eq!(style.fg, Some(Color::Black));
    }
}
