//! Application state and logic for the TUI.
//!
//! This module contains the core application state and the main run loop.
//! Integrates with mbr-core services for Metabase data access.
//!
//! ## Module Structure
//! - `mod.rs`: App struct definition, initialization, and rendering
//! - `action_handler.rs`: AppAction event processing
//! - `data_handler.rs`: Async data loading with tokio tasks
//! - `input_handler.rs`: Keyboard event processing

mod action_handler;
mod data_handler;
mod input_handler;

use std::sync::Arc;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use tokio::sync::mpsc;

use crate::action::{AppAction, DataRequest};
use crate::components::{
    ActiveTab, Component, ContentPanel, ContentView, HelpOverlay, RecordDetailOverlay, StatusBar,
};
use crate::event::{Event, EventHandler};
use crate::layout::main::{HEADER_HEIGHT, STATUS_BAR_HEIGHT};
use crate::service::{AppData, ConnectionStatus, LoadState, ServiceClient, init_service};

/// The main application state.
pub struct App {
    /// Whether the application should quit
    pub should_quit: bool,
    /// Currently active tab
    pub(crate) active_tab: ActiveTab,
    /// Content panel (full width)
    pub(crate) content: ContentPanel,
    /// Status bar (bottom)
    pub(crate) status_bar: StatusBar,
    /// Service client for API access (Arc-wrapped for async sharing)
    pub(crate) service: Option<Arc<ServiceClient>>,
    /// Connection status
    pub(crate) connection_status: ConnectionStatus,
    /// Application data from API
    pub(crate) data: AppData,
    /// Action sender for async operations
    pub(crate) action_tx: mpsc::UnboundedSender<AppAction>,
    /// Action receiver for processing
    action_rx: mpsc::UnboundedReceiver<AppAction>,
    /// Whether to show help overlay
    pub(crate) show_help: bool,
    /// Current query request ID for race condition prevention
    pub(crate) current_request_id: u64,
    /// Whether to show record detail overlay
    pub(crate) show_record_detail: bool,
    /// Record detail overlay state
    pub(crate) record_detail: Option<RecordDetailOverlay>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Check if any modal is active that should block global navigation.
    ///
    /// When a modal (sort, filter, or search) is active, global shortcuts
    /// like tab switching (1/2/3), help (?), and Tab should be blocked to prevent
    /// accidental navigation while the user is focused on the modal.
    pub(crate) fn is_modal_active(&self) -> bool {
        self.content.is_sort_mode_active()
            || self.content.is_filter_mode_active()
            || self.content.is_result_search_active()
    }

    /// Create a new application instance.
    pub fn new() -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        // Initialize service client
        let (service, connection_status) = match init_service() {
            Ok(client) => {
                let status = if client.is_authenticated() {
                    ConnectionStatus::Connecting
                } else {
                    ConnectionStatus::Disconnected
                };
                (Some(client), status)
            }
            Err(e) => (None, ConnectionStatus::Error(e)),
        };

        // Set initial view to Questions
        let mut content = ContentPanel::new();
        content.set_view(ContentView::Questions);

        Self {
            should_quit: false,
            active_tab: ActiveTab::Questions,
            content,
            status_bar: StatusBar::new(),
            service,
            connection_status,
            data: AppData::default(),
            action_tx,
            action_rx,
            show_help: false,
            current_request_id: 0,
            show_record_detail: false,
            record_detail: None,
        }
    }

    /// Run the main application loop (async version).
    pub async fn run_async(
        &mut self,
        terminal: &mut ratatui::Terminal<impl ratatui::backend::Backend>,
    ) -> std::io::Result<()> {
        let event_handler = EventHandler::new(250);

        // Validate authentication on startup if we have a service client
        if let Some(service) = &self.service {
            if service.is_authenticated() {
                self.validate_auth_async().await;
            }
        }

        // Auto-load Questions data on startup (initial view is Questions)
        if self.content.current_view() == ContentView::Questions
            && matches!(self.data.questions, LoadState::Idle)
        {
            let _ = self
                .action_tx
                .send(AppAction::LoadData(DataRequest::Questions));
        }

        while !self.should_quit {
            // Process any pending actions
            self.process_actions();

            // Draw the UI
            terminal.draw(|frame| self.draw(frame))?;

            // Handle events
            match event_handler.next()? {
                Event::Key(key) => self.handle_key(key.code, key.modifiers),
                Event::Resize(_, _) => {} // Terminal will redraw automatically
                Event::Tick => {}         // Can be used for animations/updates
            }
        }

        Ok(())
    }

    /// Process pending actions from the action queue.
    fn process_actions(&mut self) {
        while let Ok(action) = self.action_rx.try_recv() {
            self.handle_action(action);
        }
    }

    /// Validate authentication asynchronously.
    async fn validate_auth_async(&mut self) {
        if let Some(service) = &self.service {
            match service.validate_auth().await {
                Ok(user) => {
                    let display_name = user
                        .common_name
                        .clone()
                        .or_else(|| user.first_name.clone())
                        .unwrap_or_else(|| user.email.clone());
                    self.connection_status = ConnectionStatus::Connected(display_name.clone());
                    self.status_bar
                        .set_message(format!("Connected as {}", display_name));
                    self.data.current_user = Some(user);
                }
                Err(e) => {
                    self.connection_status = ConnectionStatus::Error(e.clone());
                    self.status_bar.set_message(format!("Auth failed: {}", e));
                }
            }
        }
    }

    /// Switch to a specific tab and update content view.
    pub(crate) fn switch_to_tab(&mut self, tab: ActiveTab) {
        self.active_tab = tab;
        let view = match tab {
            ActiveTab::Questions => ContentView::Questions,
            ActiveTab::Collections => ContentView::Collections,
            ActiveTab::Databases => ContentView::Databases,
        };
        self.content.set_view(view);

        // Auto-load data when switching to a view with Idle state
        match tab {
            ActiveTab::Questions => {
                if matches!(self.data.questions, LoadState::Idle) {
                    let _ = self
                        .action_tx
                        .send(AppAction::LoadData(DataRequest::Questions));
                }
            }
            ActiveTab::Collections => {
                if matches!(self.data.collections, LoadState::Idle) {
                    let _ = self
                        .action_tx
                        .send(AppAction::LoadData(DataRequest::Collections));
                }
            }
            ActiveTab::Databases => {
                if matches!(self.data.databases, LoadState::Idle) {
                    let _ = self
                        .action_tx
                        .send(AppAction::LoadData(DataRequest::Databases));
                }
            }
        }

        self.status_bar
            .set_message(format!("Viewing: {}", tab.label()));
    }

    // =========================================================================
    // Drawing
    // =========================================================================

    /// Draw the UI.
    fn draw(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Create main layout: Header with tabs, Content (100% width), Status bar
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(HEADER_HEIGHT),
                Constraint::Min(0), // Main content (100% width)
                Constraint::Length(STATUS_BAR_HEIGHT),
            ])
            .split(size);

        // Draw header with integrated tabs
        self.draw_header_with_tabs(frame, main_chunks[0]);

        // Draw content panel (full width, always focused)
        self.content.draw(frame, main_chunks[1], true);

        // Draw status bar
        self.status_bar.draw(frame, main_chunks[2], false);

        // Draw help overlay if visible
        if self.show_help {
            HelpOverlay::render(frame, size);
        }

        // Draw record detail overlay if visible
        if self.show_record_detail {
            if let Some(ref mut detail) = self.record_detail {
                detail.render(frame, size);
            }
        }
    }

    /// Draw the header with integrated tab bar.
    fn draw_header_with_tabs(&self, frame: &mut Frame, area: Rect) {
        // Build connection indicator
        let connection_indicator = match &self.connection_status {
            ConnectionStatus::Disconnected => {
                Span::styled(" ○ ", Style::default().fg(Color::DarkGray))
            }
            ConnectionStatus::Connecting => Span::styled(" ◐ ", Style::default().fg(Color::Yellow)),
            ConnectionStatus::Connected(_) => {
                Span::styled(" ● ", Style::default().fg(Color::Green))
            }
            ConnectionStatus::Error(_) => Span::styled(" ✗ ", Style::default().fg(Color::Red)),
        };

        // Build tab bar
        let tabs = [
            ActiveTab::Questions,
            ActiveTab::Collections,
            ActiveTab::Databases,
        ];
        let mut tab_spans: Vec<Span> = vec![Span::raw(" ")];

        for (i, tab) in tabs.iter().enumerate() {
            let is_active = *tab == self.active_tab;
            let style = if is_active {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default().fg(Color::White)
            };

            // Add tab with number key hint
            tab_spans.push(Span::styled(format!(" {} {} ", i + 1, tab.label()), style));
            tab_spans.push(Span::raw(" "));
        }

        // Add connection status at the end
        tab_spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
        tab_spans.push(connection_indicator);

        let header = Paragraph::new(Line::from(tab_spans)).block(
            Block::default()
                .title(" mbr-tui ")
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(header, area);
    }
}
