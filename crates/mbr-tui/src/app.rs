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
    ActiveTab, Component, ContentPanel, ContentView, HelpOverlay, QueryResultData, StatusBar,
};
use crate::event::{Event, EventHandler};
use crate::layout::main::{HEADER_HEIGHT, STATUS_BAR_HEIGHT};
use crate::service::{AppData, ConnectionStatus, LoadState, ServiceClient, init_service};

/// The main application state.
pub struct App {
    /// Whether the application should quit
    pub should_quit: bool,
    /// Currently active tab
    active_tab: ActiveTab,
    /// Content panel (full width)
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
                self.switch_to_tab(self.active_tab.next());
            }
            AppAction::PreviousPanel => {
                self.switch_to_tab(self.active_tab.previous());
            }
            AppAction::Navigate(target) => {
                let view = match target {
                    ContentTarget::Welcome => ContentView::Welcome,
                    ContentTarget::Questions => ContentView::Questions,
                    ContentTarget::Collections => ContentView::Collections,
                    ContentTarget::Databases => ContentView::Databases,
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
            // === Query Execution (Phase 6) ===
            AppAction::ExecuteQuestion(id) => {
                self.execute_question(id);
            }
            AppAction::QueryResultLoaded(result_data) => {
                let row_count = result_data.rows.len();
                let name = result_data.question_name.clone();
                self.content.set_query_result(result_data);
                self.status_bar
                    .set_message(format!("Query '{}': {} rows", name, row_count));
            }
            AppAction::QueryFailed(error) => {
                self.status_bar
                    .set_message(format!("Query failed: {}", error));
            }
            AppAction::BackToQuestions => {
                self.content.back_to_questions();
                self.status_bar.set_message("Returned to Questions list");
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
            DataRequest::Execute(id) => {
                // Execute question query - handled by execute_question method
                self.status_bar
                    .set_message(format!("Executing query #{}...", id));
                // Actual execution is handled through ExecuteQuestion action
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

    /// Execute a question query
    fn execute_question(&mut self, id: u32) {
        // Check if we have a service client
        let service = match &self.service {
            Some(s) => Arc::clone(s),
            None => {
                self.status_bar
                    .set_message("Error: Not connected to Metabase");
                return;
            }
        };

        // Get question name from loaded questions
        let question_name = self
            .data
            .questions
            .data()
            .and_then(|qs| qs.iter().find(|q| q.id == id))
            .map(|q| q.name.clone())
            .unwrap_or_else(|| format!("Question #{}", id));

        self.status_bar
            .set_message(format!("Executing '{}'...", question_name));

        let tx = self.action_tx.clone();

        tokio::spawn(async move {
            match service.execute_question(id).await {
                Ok(result) => {
                    // Convert QueryResult to QueryResultData (TUI-friendly format)
                    let columns: Vec<String> = result
                        .data
                        .cols
                        .iter()
                        .map(|c| c.display_name.clone())
                        .collect();

                    let rows: Vec<Vec<String>> = result
                        .data
                        .rows
                        .iter()
                        .map(|row| {
                            row.iter()
                                .map(|v| match v {
                                    serde_json::Value::Null => "—".to_string(),
                                    serde_json::Value::Bool(b) => b.to_string(),
                                    serde_json::Value::Number(n) => n.to_string(),
                                    serde_json::Value::String(s) => s.clone(),
                                    _ => v.to_string(),
                                })
                                .collect()
                        })
                        .collect();

                    let result_data = QueryResultData {
                        question_id: id,
                        question_name,
                        columns,
                        rows,
                    };

                    let _ = tx.send(AppAction::QueryResultLoaded(result_data));
                }
                Err(e) => {
                    let _ = tx.send(AppAction::QueryFailed(e));
                }
            }
        });
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
                // If viewing query result, go back to Questions instead of quitting
                if self.content.current_view() == ContentView::QueryResult {
                    let _ = self.action_tx.send(AppAction::BackToQuestions);
                } else {
                    self.should_quit = true;
                }
                return;
            }
            KeyCode::Char('?') => {
                self.show_help = true;
                return;
            }
            // Tab switching with number keys 1/2/3
            KeyCode::Char('1') => {
                self.switch_to_tab(ActiveTab::Questions);
                return;
            }
            KeyCode::Char('2') => {
                self.switch_to_tab(ActiveTab::Collections);
                return;
            }
            KeyCode::Char('3') => {
                self.switch_to_tab(ActiveTab::Databases);
                return;
            }
            // Tab cycling with Tab/Shift+Tab
            KeyCode::Tab => {
                let new_tab = if modifiers.contains(KeyModifiers::SHIFT) {
                    self.active_tab.previous()
                } else {
                    self.active_tab.next()
                };
                self.switch_to_tab(new_tab);
                return;
            }
            KeyCode::BackTab => {
                self.switch_to_tab(self.active_tab.previous());
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

        // Content panel keybindings
        // Handle Enter in Questions view to execute query
        if code == KeyCode::Enter && self.content.current_view() == ContentView::Questions {
            if let Some(question_id) = self.content.get_selected_question_id() {
                let _ = self.action_tx.send(AppAction::ExecuteQuestion(question_id));
                return;
            }
        }
        self.content
            .handle_key(crossterm::event::KeyEvent::new(code, modifiers));
    }

    /// Switch to a specific tab and update content view.
    fn switch_to_tab(&mut self, tab: ActiveTab) {
        self.active_tab = tab;
        let view = match tab {
            ActiveTab::Questions => ContentView::Questions,
            ActiveTab::Collections => ContentView::Collections,
            ActiveTab::Databases => ContentView::Databases,
        };
        self.content.set_view(view);

        // Auto-load data when switching to Questions view
        if view == ContentView::Questions && matches!(self.data.questions, LoadState::Idle) {
            let _ = self
                .action_tx
                .send(AppAction::LoadData(DataRequest::Questions));
        }

        self.status_bar
            .set_message(format!("Viewing: {}", tab.label()));
    }

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
    }

    /// Draw the header with integrated tab bar.
    fn draw_header_with_tabs(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
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
