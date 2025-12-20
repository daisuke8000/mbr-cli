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
    ActiveTab, Component, ContentPanel, ContentView, HelpOverlay, InputMode, QueryResultData,
    RecordDetailOverlay, StatusBar,
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
    /// Current query request ID for race condition prevention
    current_request_id: u64,
    /// Whether to show record detail overlay
    show_record_detail: bool,
    /// Record detail overlay state
    record_detail: Option<RecordDetailOverlay>,
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
            AppAction::CollectionsLoaded(collections) => {
                let count = collections.len();
                self.data.collections = LoadState::Loaded(collections);
                // Sync to content panel for display
                self.content.update_collections(&self.data.collections);
                self.status_bar
                    .set_message(format!("Loaded {} collections", count));
            }
            AppAction::DatabasesLoaded(databases) => {
                let count = databases.len();
                self.data.databases = LoadState::Loaded(databases);
                // Sync to content panel for display
                self.content.update_databases(&self.data.databases);
                self.status_bar
                    .set_message(format!("Loaded {} databases", count));
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
            AppAction::LoadFailed(request, error) => {
                // Set error state on the appropriate data based on request type
                match request {
                    DataRequest::Questions
                    | DataRequest::Refresh
                    | DataRequest::SearchQuestions(_)
                    | DataRequest::FilterQuestionsByCollection(_) => {
                        self.data.questions = LoadState::Error(error.clone());
                        self.content.update_questions(&self.data.questions);
                    }
                    DataRequest::Collections => {
                        self.data.collections = LoadState::Error(error.clone());
                        self.content.update_collections(&self.data.collections);
                    }
                    DataRequest::Databases => {
                        self.data.databases = LoadState::Error(error.clone());
                        self.content.update_databases(&self.data.databases);
                    }
                    _ => {} // QuestionDetails and Execute don't use LoadState
                }
                self.status_bar.set_message(format!("Error: {}", error));
            }
            // === Query Execution (Phase 6) ===
            AppAction::ExecuteQuestion(id) => {
                self.execute_question(id);
            }
            AppAction::QueryResultLoaded(request_id, result_data) => {
                // Only process if this is the current request (ignore stale results)
                if request_id == self.current_request_id {
                    let row_count = result_data.rows.len();
                    let name = result_data.question_name.clone();
                    // Store in centralized App.data
                    self.data.query_result = Some(result_data.clone());
                    // Sync to ContentPanel for display
                    self.content.set_query_result(result_data);
                    self.status_bar
                        .set_message(format!("Query '{}': {} rows", name, row_count));
                }
            }
            AppAction::QueryFailed(request_id, error) => {
                // Only process if this is the current request (ignore stale errors)
                if request_id == self.current_request_id {
                    self.status_bar
                        .set_message(format!("Query failed: {}", error));
                }
            }
            AppAction::BackToQuestions => {
                // Clear centralized query result data
                self.data.query_result = None;
                self.content.back_to_questions();
                self.status_bar.set_message("Returned to Questions list");
            }
            // === Collection Drill-down (Phase 3) ===
            AppAction::DrillDownCollection(collection_id, collection_name) => {
                // Enter collection questions view
                self.content
                    .enter_collection_questions(collection_id, collection_name.clone());
                self.status_bar
                    .set_message(format!("Viewing questions in '{}'", collection_name));
                // Trigger data load
                let _ = self.action_tx.send(AppAction::LoadData(
                    DataRequest::FilterQuestionsByCollection(collection_id),
                ));
            }
            AppAction::BackToCollections => {
                // Exit collection questions view
                self.content.exit_collection_questions();
                self.status_bar.set_message("Returned to Collections list");
            }

            // === Database Drill-down ===
            AppAction::DrillDownDatabase(database_id, database_name) => {
                // Enter database schemas view
                self.content
                    .enter_database_schemas(database_id, database_name.clone());
                self.status_bar
                    .set_message(format!("Viewing schemas in '{}'", database_name));
                // Trigger data load
                let _ = self
                    .action_tx
                    .send(AppAction::LoadData(DataRequest::Schemas(database_id)));
            }
            AppAction::BackToDatabases => {
                // Exit database schemas view
                self.content.exit_database_schemas();
                self.status_bar.set_message("Returned to Databases list");
            }
            AppAction::DrillDownSchema(schema_name) => {
                // Get database_id from context (clone to avoid borrow conflicts)
                if let Some(database_id) = self.content.get_database_context().map(|(id, _)| *id) {
                    // Enter schema tables view
                    self.content
                        .enter_schema_tables(database_id, schema_name.clone());
                    self.status_bar
                        .set_message(format!("Viewing tables in '{}'", schema_name));
                    // Trigger data load
                    let _ = self.action_tx.send(AppAction::LoadData(DataRequest::Tables(
                        database_id,
                        schema_name,
                    )));
                }
            }
            AppAction::BackToSchemas => {
                // Exit schema tables view
                self.content.exit_schema_tables();
                self.status_bar.set_message("Returned to Schemas list");
            }
            AppAction::DrillDownTable(table_id, table_name) => {
                // Get database_id from context (clone to avoid borrow conflicts)
                if let Some(database_id) = self.content.get_schema_context().map(|(id, _)| *id) {
                    // Enter table preview view
                    self.content
                        .enter_table_preview(database_id, table_id, table_name.clone());
                    self.status_bar
                        .set_message(format!("Loading preview for '{}'...", table_name));
                    // Trigger data load
                    let _ = self
                        .action_tx
                        .send(AppAction::LoadData(DataRequest::TablePreview(
                            database_id,
                            table_id,
                        )));
                }
            }
            AppAction::BackToTables => {
                // Exit table preview view
                self.content.exit_table_preview();
                self.status_bar.set_message("Returned to Tables list");
            }
            AppAction::SchemasLoaded(schemas) => {
                // Store schemas in data
                self.data.schemas = LoadState::Loaded(schemas.clone());
                // Sync to content panel
                self.content.update_schemas(&self.data.schemas);
                // Update status
                self.status_bar
                    .set_message(format!("Loaded {} schemas", schemas.len()));
            }
            AppAction::TablesLoaded(tables) => {
                // Store tables in data
                self.data.tables = LoadState::Loaded(tables.clone());
                // Sync to content panel
                self.content.update_tables(&self.data.tables);
                // Update status
                self.status_bar
                    .set_message(format!("Loaded {} tables", tables.len()));
            }
            AppAction::TablePreviewLoaded(data) => {
                // Store query result for table preview
                self.data.query_result = Some(data.clone());
                // Set the query result in content panel (uses shared rendering)
                self.content.set_table_preview_data(data.clone());
                // Update status
                self.status_bar
                    .set_message(format!("Preview: {} rows loaded", data.rows.len()));
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
                            let _ = tx.send(AppAction::LoadFailed(DataRequest::Questions, e));
                        }
                    }
                });
            }
            DataRequest::SearchQuestions(query) => {
                // Guard: prevent duplicate requests while loading
                if matches!(self.data.questions, LoadState::Loading) {
                    return;
                }

                // Set loading state
                self.data.questions = LoadState::Loading;
                // Sync to content panel for display
                self.content.update_questions(&self.data.questions);
                self.status_bar
                    .set_message(format!("Searching for '{}'...", query));

                // Spawn background task with search parameter
                tokio::spawn(async move {
                    match service.fetch_questions(Some(&query), Some(50)).await {
                        Ok(questions) => {
                            let _ = tx.send(AppAction::QuestionsLoaded(questions));
                        }
                        Err(e) => {
                            let _ = tx.send(AppAction::LoadFailed(DataRequest::Questions, e));
                        }
                    }
                });
            }
            DataRequest::FilterQuestionsByCollection(collection_id) => {
                // Guard: prevent duplicate requests while loading
                if matches!(self.data.questions, LoadState::Loading) {
                    return;
                }

                // Set loading state
                self.data.questions = LoadState::Loading;
                // Sync to content panel for display
                self.content.update_questions(&self.data.questions);

                let collection_str = collection_id.to_string();

                // Spawn background task with collection filter
                tokio::spawn(async move {
                    match service
                        .fetch_questions_by_collection(&collection_str, Some(100))
                        .await
                    {
                        Ok(questions) => {
                            let _ = tx.send(AppAction::QuestionsLoaded(questions));
                        }
                        Err(e) => {
                            let _ = tx.send(AppAction::LoadFailed(
                                DataRequest::FilterQuestionsByCollection(collection_id),
                                e,
                            ));
                        }
                    }
                });
            }
            DataRequest::Collections => {
                // Guard: prevent duplicate requests while loading
                if matches!(self.data.collections, LoadState::Loading) {
                    return;
                }

                // Set loading state
                self.data.collections = LoadState::Loading;
                // Sync to content panel for display
                self.content.update_collections(&self.data.collections);
                self.status_bar.set_message("Loading collections...");

                // Spawn background task
                tokio::spawn(async move {
                    match service.fetch_collections().await {
                        Ok(collections) => {
                            let _ = tx.send(AppAction::CollectionsLoaded(collections));
                        }
                        Err(e) => {
                            let _ = tx.send(AppAction::LoadFailed(DataRequest::Collections, e));
                        }
                    }
                });
            }
            DataRequest::Databases => {
                // Guard: prevent duplicate requests while loading
                if matches!(self.data.databases, LoadState::Loading) {
                    return;
                }

                // Set loading state
                self.data.databases = LoadState::Loading;
                // Sync to content panel for display
                self.content.update_databases(&self.data.databases);
                self.status_bar.set_message("Loading databases...");

                // Spawn background task
                tokio::spawn(async move {
                    match service.fetch_databases().await {
                        Ok(databases) => {
                            let _ = tx.send(AppAction::DatabasesLoaded(databases));
                        }
                        Err(e) => {
                            let _ = tx.send(AppAction::LoadFailed(DataRequest::Databases, e));
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
            DataRequest::Schemas(database_id) => {
                // Guard: prevent duplicate requests while loading
                if matches!(self.data.schemas, LoadState::Loading) {
                    return;
                }

                // Set loading state
                self.data.schemas = LoadState::Loading;
                // Sync to content panel for display
                self.content.update_schemas(&self.data.schemas);
                self.status_bar.set_message("Loading schemas...");

                // Spawn background task
                tokio::spawn(async move {
                    match service.fetch_schemas(database_id).await {
                        Ok(schemas) => {
                            let _ = tx.send(AppAction::SchemasLoaded(schemas));
                        }
                        Err(e) => {
                            let _ = tx
                                .send(AppAction::LoadFailed(DataRequest::Schemas(database_id), e));
                        }
                    }
                });
            }
            DataRequest::Tables(database_id, ref schema_name) => {
                // Guard: prevent duplicate requests while loading
                if matches!(self.data.tables, LoadState::Loading) {
                    return;
                }

                // Set loading state
                self.data.tables = LoadState::Loading;
                // Sync to content panel for display
                self.content.update_tables(&self.data.tables);
                self.status_bar
                    .set_message(format!("Loading tables in '{}'...", schema_name));

                let schema = schema_name.clone();

                // Spawn background task
                tokio::spawn(async move {
                    match service.fetch_tables(database_id, &schema).await {
                        Ok(tables) => {
                            let _ = tx.send(AppAction::TablesLoaded(tables));
                        }
                        Err(e) => {
                            let _ = tx.send(AppAction::LoadFailed(
                                DataRequest::Tables(database_id, schema),
                                e,
                            ));
                        }
                    }
                });
            }
            DataRequest::TablePreview(database_id, table_id) => {
                // Set status message
                self.status_bar.set_message("Loading table preview...");

                // Spawn background task
                tokio::spawn(async move {
                    match service.preview_table(database_id, table_id, 100).await {
                        Ok(result) => {
                            // Convert QueryResult to QueryResultData
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
                                question_id: table_id,
                                question_name: format!("Table #{}", table_id),
                                columns,
                                rows,
                            };

                            let _ = tx.send(AppAction::TablePreviewLoaded(result_data));
                        }
                        Err(e) => {
                            let _ = tx.send(AppAction::LoadFailed(
                                DataRequest::TablePreview(database_id, table_id),
                                e,
                            ));
                        }
                    }
                });
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

        // Increment request ID to invalidate any in-flight requests
        self.current_request_id = self.current_request_id.wrapping_add(1);
        let request_id = self.current_request_id;

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

                    let _ = tx.send(AppAction::QueryResultLoaded(request_id, result_data));
                }
                Err(e) => {
                    let _ = tx.send(AppAction::QueryFailed(request_id, e));
                }
            }
        });
    }

    /// Handle keyboard input.
    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        // Record detail overlay takes priority when shown
        if self.show_record_detail {
            match code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.show_record_detail = false;
                    self.record_detail = None;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if let Some(ref mut detail) = self.record_detail {
                        detail.scroll_up();
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(ref mut detail) = self.record_detail {
                        detail.scroll_down();
                    }
                }
                _ => {} // Ignore other keys when detail is shown
            }
            return;
        }

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

        // Search mode handling (takes priority over global keys in Questions view)
        if self.content.input_mode() == InputMode::Search {
            match code {
                KeyCode::Enter => {
                    // Execute search
                    if let Some(query) = self.content.execute_search() {
                        let _ = self
                            .action_tx
                            .send(AppAction::LoadData(DataRequest::SearchQuestions(query)));
                    } else {
                        // Empty query: reload all questions
                        let _ = self
                            .action_tx
                            .send(AppAction::LoadData(DataRequest::Questions));
                    }
                    return;
                }
                KeyCode::Esc => {
                    // Cancel search mode
                    self.content.exit_search_mode();
                    return;
                }
                _ => {
                    // Delegate to content panel for character input
                    self.content
                        .handle_key(crossterm::event::KeyEvent::new(code, modifiers));
                    return;
                }
            }
        }

        // Global keybindings (always active when not in search mode)
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
                } else if self.content.current_view() == ContentView::CollectionQuestions {
                    // Return to Collections list from collection questions view
                    let _ = self.action_tx.send(AppAction::BackToCollections);
                } else if self.content.current_view() == ContentView::DatabaseSchemas {
                    // Return to Databases list from schemas view
                    let _ = self.action_tx.send(AppAction::BackToDatabases);
                } else if self.content.current_view() == ContentView::SchemaTables {
                    // Return to Schemas list from tables view
                    let _ = self.action_tx.send(AppAction::BackToSchemas);
                } else if self.content.current_view() == ContentView::TablePreview {
                    // Return to Tables list from preview view
                    let _ = self.action_tx.send(AppAction::BackToTables);
                } else if self.content.get_active_search().is_some() {
                    // Clear active search and reload all questions
                    self.content.clear_search();
                    let _ = self
                        .action_tx
                        .send(AppAction::LoadData(DataRequest::Questions));
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
            // Refresh data with 'r' - reloads current view's data
            KeyCode::Char('r') => {
                let request = match self.content.current_view() {
                    ContentView::Questions => DataRequest::Questions,
                    ContentView::Collections => DataRequest::Collections,
                    ContentView::Databases => DataRequest::Databases,
                    _ => DataRequest::Refresh,
                };
                // Force reload by resetting state to Idle first
                match self.content.current_view() {
                    ContentView::Questions => self.data.questions = LoadState::Idle,
                    ContentView::Collections => self.data.collections = LoadState::Idle,
                    ContentView::Databases => self.data.databases = LoadState::Idle,
                    _ => {}
                }
                let _ = self.action_tx.send(AppAction::LoadData(request));
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

        // Handle Enter in CollectionQuestions view to execute query
        if code == KeyCode::Enter && self.content.current_view() == ContentView::CollectionQuestions
        {
            if let Some(question_id) = self.content.get_selected_question_id() {
                let _ = self.action_tx.send(AppAction::ExecuteQuestion(question_id));
                return;
            }
        }

        // Handle Enter in Collections view to drill down into collection
        if code == KeyCode::Enter && self.content.current_view() == ContentView::Collections {
            if let Some((collection_id, collection_name)) =
                self.content.get_selected_collection_info()
            {
                let _ = self.action_tx.send(AppAction::DrillDownCollection(
                    collection_id,
                    collection_name,
                ));
                return;
            }
        }

        // Handle Enter in QueryResult view to show record detail
        if code == KeyCode::Enter && self.content.current_view() == ContentView::QueryResult {
            if let Some((columns, values)) = self.content.get_selected_record() {
                self.record_detail = Some(RecordDetailOverlay::new(columns, values));
                self.show_record_detail = true;
                return;
            }
        }

        // Handle Enter in TablePreview view to show record detail
        if code == KeyCode::Enter && self.content.current_view() == ContentView::TablePreview {
            if let Some((columns, values)) = self.content.get_selected_record() {
                self.record_detail = Some(RecordDetailOverlay::new(columns, values));
                self.show_record_detail = true;
                return;
            }
        }

        // Handle Enter in Databases view to drill down into database schemas
        if code == KeyCode::Enter && self.content.current_view() == ContentView::Databases {
            if let Some((database_id, database_name)) = self.content.get_selected_database_info() {
                let _ = self
                    .action_tx
                    .send(AppAction::DrillDownDatabase(database_id, database_name));
                return;
            }
        }

        // Handle Enter in DatabaseSchemas view to drill down into schema tables
        if code == KeyCode::Enter && self.content.current_view() == ContentView::DatabaseSchemas {
            if let Some(schema_name) = self.content.get_selected_schema() {
                let _ = self.action_tx.send(AppAction::DrillDownSchema(schema_name));
                return;
            }
        }

        // Handle Enter in SchemaTables view to preview table data
        if code == KeyCode::Enter && self.content.current_view() == ContentView::SchemaTables {
            if let Some((table_id, table_name)) = self.content.get_selected_table_info() {
                let _ = self
                    .action_tx
                    .send(AppAction::DrillDownTable(table_id, table_name));
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

        // Auto-load data when switching to a view with Idle state
        match view {
            ContentView::Questions => {
                if matches!(self.data.questions, LoadState::Idle) {
                    let _ = self
                        .action_tx
                        .send(AppAction::LoadData(DataRequest::Questions));
                }
            }
            ContentView::Collections => {
                if matches!(self.data.collections, LoadState::Idle) {
                    let _ = self
                        .action_tx
                        .send(AppAction::LoadData(DataRequest::Collections));
                }
            }
            ContentView::Databases => {
                if matches!(self.data.databases, LoadState::Idle) {
                    let _ = self
                        .action_tx
                        .send(AppAction::LoadData(DataRequest::Databases));
                }
            }
            _ => {}
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

        // Draw record detail overlay if visible
        if self.show_record_detail {
            if let Some(ref detail) = self.record_detail {
                detail.render(frame, size);
            }
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
