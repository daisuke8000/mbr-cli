//! Application state and logic for the TUI.
//!
//! This module contains the core application state and the main run loop.
//! Integrates with mbr-core services for Metabase data access.

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use tokio::sync::mpsc;

use crate::action::{AppAction, ContentTarget, DataRequest};
use crate::components::{
    ActivePanel, Component, ContentPanel, ContentView, HelpOverlay, NavigationPanel, StatusBar,
};
use crate::event::{Event, EventHandler};
use crate::layout::main::{
    CONTENT_PANEL_WIDTH_PERCENT, HEADER_HEIGHT, NAV_PANEL_WIDTH_PERCENT, STATUS_BAR_HEIGHT,
};
use crate::service::{AppData, ConnectionStatus, LoadState, ServiceClient, init_service};

/// The main application state.
pub struct App {
    /// Whether the application should quit
    pub should_quit: bool,
    /// Currently active panel
    active_panel: ActivePanel,
    /// Navigation panel (left)
    navigation: NavigationPanel,
    /// Content panel (right)
    content: ContentPanel,
    /// Status bar (bottom)
    status_bar: StatusBar,
    /// Service client for API access (Arc-wrapped for async sharing)
    service: Option<Arc<ServiceClient>>,
    /// Connection status
    connection_status: ConnectionStatus,
    /// Application data from API
    data: AppData,
    /// Action sender for async operations
    action_tx: mpsc::UnboundedSender<AppAction>,
    /// Action receiver for processing
    action_rx: mpsc::UnboundedReceiver<AppAction>,
    /// Whether to show help overlay
    show_help: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
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

        Self {
            should_quit: false,
            active_panel: ActivePanel::Navigation,
            navigation: NavigationPanel::new(),
            content: ContentPanel::new(),
            status_bar: StatusBar::new(),
            service,
            connection_status,
            data: AppData::default(),
            action_tx,
            action_rx,
            show_help: false,
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

    /// Process pending actions from the action queue
    fn process_actions(&mut self) {
        while let Ok(action) = self.action_rx.try_recv() {
            self.handle_action(action);
        }
    }

    /// Handle an application action
    fn handle_action(&mut self, action: AppAction) {
        match action {
            AppAction::Quit => {
                self.should_quit = true;
            }
            AppAction::NextPanel => {
                self.active_panel = self.active_panel.next();
            }
            AppAction::PreviousPanel => {
                self.active_panel = self.active_panel.previous();
            }
            AppAction::Navigate(target) => {
                let view = match target {
                    ContentTarget::Welcome => ContentView::Welcome,
                    ContentTarget::Questions => ContentView::Questions,
                    ContentTarget::Collections => ContentView::Collections,
                    ContentTarget::Databases => ContentView::Databases,
                    ContentTarget::Settings => ContentView::Settings,
                };
                self.content.set_view(view);
            }
            AppAction::LoadData(request) => {
                self.handle_data_request(request);
            }
            AppAction::ShowError(msg) => {
                self.data.questions = LoadState::Error(msg.clone());
                self.status_bar.set_message(format!("Error: {}", msg));
            }
            AppAction::ClearError => {
                // Reset to Idle state when clearing error
                if self.data.questions.is_error() {
                    self.data.questions = LoadState::Idle;
                }
            }
            AppAction::SetStatus(msg) => {
                self.status_bar.set_message(msg);
            }
            AppAction::ClearStatus => {
                self.status_bar.set_message("");
            }
            // === Completion Notifications ===
            AppAction::QuestionsLoaded(questions) => {
                let count = questions.len();
                self.data.questions = LoadState::Loaded(questions);
                // Sync to content panel for display
                self.content.update_questions(&self.data.questions);
                self.status_bar
                    .set_message(format!("Loaded {} questions", count));
            }
            AppAction::AuthValidated(user) => {
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
            AppAction::LoadFailed(error) => {
                self.data.questions = LoadState::Error(error.clone());
                // Sync to content panel for display
                self.content.update_questions(&self.data.questions);
                self.status_bar.set_message(format!("Error: {}", error));
            }
        }
    }

    /// Handle data loading request with background task spawning
    fn handle_data_request(&mut self, request: DataRequest) {
        // Check if we have a service client
        let service = match &self.service {
            Some(s) => Arc::clone(s),
            None => {
                self.status_bar
                    .set_message("Error: Not connected to Metabase");
                return;
            }
        };

        let tx = self.action_tx.clone();

        match request {
            DataRequest::Questions | DataRequest::Refresh => {
                // Guard: prevent duplicate requests while loading
                if matches!(self.data.questions, LoadState::Loading) {
                    return;
                }

                // Set loading state
                self.data.questions = LoadState::Loading;
                // Sync to content panel for display
                self.content.update_questions(&self.data.questions);
                self.status_bar.set_message("Loading questions...");

                // Spawn background task
                tokio::spawn(async move {
                    match service.fetch_questions(None, Some(50)).await {
                        Ok(questions) => {
                            let _ = tx.send(AppAction::QuestionsLoaded(questions));
                        }
                        Err(e) => {
                            let _ = tx.send(AppAction::LoadFailed(e));
                        }
                    }
                });
            }
            DataRequest::QuestionDetails(id) => {
                // Question details loading - placeholder for future implementation
                self.status_bar
                    .set_message(format!("Loading question #{}...", id));
            }
        }
    }

    /// Validate authentication asynchronously
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

    /// Handle keyboard input.
    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        // Help overlay takes priority when shown
        if self.show_help {
            match code {
                KeyCode::Char('?') | KeyCode::Esc => {
                    self.show_help = false;
                }
                _ => {} // Ignore other keys when help is shown
            }
            return;
        }

        // Global keybindings (always active)
        match code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.should_quit = true;
                return;
            }
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return;
            }
            KeyCode::Esc => {
                self.should_quit = true;
                return;
            }
            KeyCode::Char('?') => {
                self.show_help = true;
                return;
            }
            KeyCode::Tab => {
                self.active_panel = if modifiers.contains(KeyModifiers::SHIFT) {
                    self.active_panel.previous()
                } else {
                    self.active_panel.next()
                };
                return;
            }
            KeyCode::BackTab => {
                self.active_panel = self.active_panel.previous();
                return;
            }
            // Refresh data with 'r'
            KeyCode::Char('r') => {
                let _ = self
                    .action_tx
                    .send(AppAction::LoadData(DataRequest::Refresh));
                return;
            }
            _ => {}
        }

        // Panel-specific keybindings
        match self.active_panel {
            ActivePanel::Navigation => {
                // Handle Enter to switch content view
                if code == KeyCode::Enter {
                    self.handle_navigation_select();
                    return;
                }
                self.navigation
                    .handle_key(crossterm::event::KeyEvent::new(code, modifiers));
            }
            ActivePanel::Content => {
                self.content
                    .handle_key(crossterm::event::KeyEvent::new(code, modifiers));
            }
        }
    }

    /// Handle navigation item selection.
    fn handle_navigation_select(&mut self) {
        let view = match self.navigation.selected() {
            0 => ContentView::Questions,
            1 => ContentView::Collections,
            2 => ContentView::Databases,
            3 => ContentView::Settings,
            _ => ContentView::Welcome,
        };
        self.content.set_view(view);

        // Update status message
        if let Some(item) = self.navigation.selected_item() {
            self.status_bar
                .set_message(format!("Viewing: {}", item.label));
        }
    }

    /// Draw the UI.
    fn draw(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Create main layout: Header, Main, Footer
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(HEADER_HEIGHT),
                Constraint::Min(0), // Main content
                Constraint::Length(STATUS_BAR_HEIGHT),
            ])
            .split(size);

        // Draw header
        self.draw_header(frame, main_chunks[0]);

        // Split main area into navigation (left) and content (right)
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(NAV_PANEL_WIDTH_PERCENT),
                Constraint::Percentage(CONTENT_PANEL_WIDTH_PERCENT),
            ])
            .split(main_chunks[1]);

        // Draw panels with focus state
        self.navigation.draw(
            frame,
            content_chunks[0],
            self.active_panel == ActivePanel::Navigation,
        );
        self.content.draw(
            frame,
            content_chunks[1],
            self.active_panel == ActivePanel::Content,
        );

        // Draw status bar
        self.status_bar.draw(frame, main_chunks[2], false);

        // Draw help overlay if visible
        if self.show_help {
            HelpOverlay::render(frame, size);
        }
    }

    /// Draw the header with connection status.
    fn draw_header(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let connection_indicator = match &self.connection_status {
            ConnectionStatus::Disconnected => {
                Span::styled(" ○ Disconnected ", Style::default().fg(Color::DarkGray))
            }
            ConnectionStatus::Connecting => {
                Span::styled(" ◐ Connecting... ", Style::default().fg(Color::Yellow))
            }
            ConnectionStatus::Connected(name) => {
                Span::styled(format!(" ● {} ", name), Style::default().fg(Color::Green))
            }
            ConnectionStatus::Error(_) => {
                Span::styled(" ✗ Error ", Style::default().fg(Color::Red))
            }
        };

        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                " mbr-tui ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("- Metabase Terminal UI "),
            Span::styled("│", Style::default().fg(Color::DarkGray)),
            connection_indicator,
            Span::styled("│", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(" Active: {} ", self.active_panel_name()),
                Style::default().fg(Color::Yellow),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(header, area);
    }

    /// Get the name of the active panel.
    fn active_panel_name(&self) -> &str {
        match self.active_panel {
            ActivePanel::Navigation => "Navigation",
            ActivePanel::Content => "Content",
        }
    }
}
