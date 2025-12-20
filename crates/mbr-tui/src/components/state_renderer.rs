//! LoadState rendering helpers.
//!
//! Provides common rendering patterns for LoadState-based data views,
//! eliminating code duplication across different view types.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::service::LoadState;

/// Configuration for rendering LoadState views.
pub struct LoadStateConfig<'a> {
    /// Title for the view
    pub title: &'a str,
    /// Message shown in Idle state
    pub idle_message: &'a str,
    /// Message shown in Loading state
    pub loading_message: &'a str,
    /// Border style based on focus state
    pub border_style: Style,
}

impl<'a> LoadStateConfig<'a> {
    /// Create a new LoadStateConfig with the given title and focused state.
    pub fn new(title: &'a str, focused: bool) -> Self {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        Self {
            title,
            idle_message: "Press 'r' to load data",
            loading_message: "Loading...",
            border_style,
        }
    }

    /// Set a custom idle message.
    pub fn with_idle_message(mut self, message: &'a str) -> Self {
        self.idle_message = message;
        self
    }

    /// Set a custom loading message.
    pub fn with_loading_message(mut self, message: &'a str) -> Self {
        self.loading_message = message;
        self
    }
}

/// Render the Idle state placeholder.
pub fn render_idle(frame: &mut Frame, area: Rect, config: &LoadStateConfig<'_>) {
    let paragraph = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", config.idle_message),
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::default()
            .title(config.title)
            .borders(Borders::ALL)
            .border_style(config.border_style),
    );
    frame.render_widget(paragraph, area);
}

/// Render the Loading state placeholder.
pub fn render_loading(frame: &mut Frame, area: Rect, config: &LoadStateConfig<'_>) {
    let paragraph = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  ⏳ {}", config.loading_message),
            Style::default().fg(Color::Yellow),
        )),
    ])
    .block(
        Block::default()
            .title(config.title)
            .borders(Borders::ALL)
            .border_style(config.border_style),
    );
    frame.render_widget(paragraph, area);
}

/// Render the Error state with message and retry hint.
pub fn render_error(frame: &mut Frame, area: Rect, config: &LoadStateConfig<'_>, error_msg: &str) {
    let paragraph = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  ❌ Error: {}", error_msg),
            Style::default().fg(Color::Red),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Press 'r' to retry",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::default()
            .title(config.title)
            .borders(Borders::ALL)
            .border_style(config.border_style),
    );
    frame.render_widget(paragraph, area);
}

/// Render an empty state placeholder with a custom message.
pub fn render_empty(
    frame: &mut Frame,
    area: Rect,
    config: &LoadStateConfig<'_>,
    empty_message: &str,
) {
    let paragraph = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", empty_message),
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::default()
            .title(config.title)
            .borders(Borders::ALL)
            .border_style(config.border_style),
    );
    frame.render_widget(paragraph, area);
}

/// Render an empty state with custom message and hint.
#[allow(dead_code)] // Reserved for future views that need hint messages
pub fn render_empty_with_hint(
    frame: &mut Frame,
    area: Rect,
    config: &LoadStateConfig<'_>,
    empty_message: &str,
    hint_message: &str,
) {
    let paragraph = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", empty_message),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", hint_message),
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::default()
            .title(config.title)
            .borders(Borders::ALL)
            .border_style(config.border_style),
    );
    frame.render_widget(paragraph, area);
}

/// Render the non-Loaded states (Idle, Loading, Error) for a LoadState.
/// Returns true if the state was handled (not Loaded), false if Loaded.
///
/// This is the primary helper function for reducing LoadState boilerplate.
/// Usage:
/// ```ignore
/// if render_non_loaded_state(frame, area, &self.data, &config) {
///     return; // Non-loaded state was rendered
/// }
/// // Handle Loaded case here
/// ```
pub fn render_non_loaded_state<T>(
    frame: &mut Frame,
    area: Rect,
    state: &LoadState<T>,
    config: &LoadStateConfig<'_>,
) -> bool {
    match state {
        LoadState::Idle => {
            render_idle(frame, area, config);
            true
        }
        LoadState::Loading => {
            render_loading(frame, area, config);
            true
        }
        LoadState::Error(msg) => {
            render_error(frame, area, config, msg);
            true
        }
        LoadState::Loaded(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_state_config_new() {
        let config = LoadStateConfig::new(" Test ", true);
        assert_eq!(config.title, " Test ");
        assert_eq!(config.idle_message, "Press 'r' to load data");
        assert_eq!(config.loading_message, "Loading...");
        assert_eq!(config.border_style.fg, Some(Color::Cyan));
    }

    #[test]
    fn test_load_state_config_unfocused() {
        let config = LoadStateConfig::new(" Test ", false);
        assert_eq!(config.border_style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn test_load_state_config_with_messages() {
        let config = LoadStateConfig::new(" Test ", true)
            .with_idle_message("Custom idle")
            .with_loading_message("Custom loading");
        assert_eq!(config.idle_message, "Custom idle");
        assert_eq!(config.loading_message, "Custom loading");
    }
}
