//! Application action handling.
//!
//! Processes AppAction events and updates application state accordingly.

use crate::action::{AppAction, ContentTarget, DataRequest};
use crate::components::{ContentView, QueryResultData};
use crate::service::LoadState;

use super::App;

impl App {
    /// Handle an application action.
    pub(super) fn handle_action(&mut self, action: AppAction) {
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
                self.handle_questions_loaded(questions);
            }
            AppAction::CollectionsLoaded(collections) => {
                self.handle_collections_loaded(collections);
            }
            AppAction::DatabasesLoaded(databases) => {
                self.handle_databases_loaded(databases);
            }
            AppAction::AuthValidated(user) => {
                self.handle_auth_validated(user);
            }
            AppAction::LoadFailed(request, error) => {
                self.handle_load_failed(request, error);
            }
            // === Query Execution ===
            AppAction::ExecuteQuestion(id) => {
                self.execute_question(id);
            }
            AppAction::QueryResultLoaded(request_id, result_data) => {
                self.handle_query_result_loaded(request_id, result_data);
            }
            AppAction::QueryFailed(request_id, error) => {
                self.handle_query_failed(request_id, error);
            }
            AppAction::BackToQuestions => {
                self.data.query_result = None;
                self.content.back_to_questions();
                self.status_bar.set_message("Returned to Questions list");
            }
            // === Collection Drill-down ===
            AppAction::DrillDownCollection(collection_id, collection_name) => {
                self.handle_drill_down_collection(collection_id, collection_name);
            }
            AppAction::BackToCollections => {
                self.content.exit_collection_questions();
                self.status_bar.set_message("Returned to Collections list");
            }
            // === Database Drill-down ===
            AppAction::DrillDownDatabase(database_id, database_name) => {
                self.handle_drill_down_database(database_id, database_name);
            }
            AppAction::BackToDatabases => {
                self.content.exit_database_schemas();
                self.status_bar.set_message("Returned to Databases list");
            }
            AppAction::DrillDownSchema(schema_name) => {
                self.handle_drill_down_schema(schema_name);
            }
            AppAction::BackToSchemas => {
                self.content.exit_schema_tables();
                self.status_bar.set_message("Returned to Schemas list");
            }
            AppAction::DrillDownTable(table_id, table_name) => {
                self.handle_drill_down_table(table_id, table_name);
            }
            AppAction::BackToTables => {
                self.content.exit_table_preview();
                self.status_bar.set_message("Returned to Tables list");
            }
            AppAction::SchemasLoaded(schemas) => {
                self.handle_schemas_loaded(schemas);
            }
            AppAction::TablesLoaded(tables) => {
                self.handle_tables_loaded(tables);
            }
            AppAction::TablePreviewLoaded(data) => {
                self.handle_table_preview_loaded(data);
            }
        }
    }

    // === Action Handlers ===

    fn handle_questions_loaded(&mut self, questions: Vec<mbr_core::api::models::Question>) {
        let count = questions.len();
        self.data.questions = LoadState::Loaded(questions);
        self.content.update_questions(&self.data.questions);
        self.status_bar
            .set_message(format!("Loaded {} questions", count));
    }

    fn handle_collections_loaded(
        &mut self,
        collections: Vec<mbr_core::api::models::CollectionItem>,
    ) {
        let count = collections.len();
        self.data.collections = LoadState::Loaded(collections);
        self.content.update_collections(&self.data.collections);
        self.status_bar
            .set_message(format!("Loaded {} collections", count));
    }

    fn handle_databases_loaded(&mut self, databases: Vec<mbr_core::api::models::Database>) {
        let count = databases.len();
        self.data.databases = LoadState::Loaded(databases);
        self.content.update_databases(&self.data.databases);
        self.status_bar
            .set_message(format!("Loaded {} databases", count));
    }

    fn handle_auth_validated(&mut self, user: mbr_core::api::models::CurrentUser) {
        let display_name = user
            .common_name
            .clone()
            .or_else(|| user.first_name.clone())
            .unwrap_or_else(|| user.email.clone());
        self.connection_status = crate::service::ConnectionStatus::Connected(display_name.clone());
        self.status_bar
            .set_message(format!("Connected as {}", display_name));
        self.data.current_user = Some(user);
    }

    fn handle_load_failed(&mut self, request: DataRequest, error: String) {
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
            _ => {}
        }
        self.status_bar.set_message(format!("Error: {}", error));
    }

    fn handle_query_result_loaded(&mut self, request_id: u64, result_data: QueryResultData) {
        if request_id == self.current_request_id {
            let row_count = result_data.rows.len();
            let name = result_data.question_name.clone();
            self.data.query_result = Some(result_data.clone());
            self.content.set_query_result(result_data);
            self.status_bar
                .set_message(format!("Query '{}': {} rows", name, row_count));
        }
    }

    fn handle_query_failed(&mut self, request_id: u64, error: String) {
        if request_id == self.current_request_id {
            self.status_bar
                .set_message(format!("Query failed: {}", error));
        }
    }

    fn handle_drill_down_collection(&mut self, collection_id: u32, collection_name: String) {
        self.content
            .enter_collection_questions(collection_id, collection_name.clone());
        self.status_bar
            .set_message(format!("Viewing questions in '{}'", collection_name));
        let _ = self.action_tx.send(AppAction::LoadData(
            DataRequest::FilterQuestionsByCollection(collection_id),
        ));
    }

    fn handle_drill_down_database(&mut self, database_id: u32, database_name: String) {
        self.content
            .enter_database_schemas(database_id, database_name.clone());
        self.status_bar
            .set_message(format!("Viewing schemas in '{}'", database_name));
        let _ = self
            .action_tx
            .send(AppAction::LoadData(DataRequest::Schemas(database_id)));
    }

    fn handle_drill_down_schema(&mut self, schema_name: String) {
        if let Some(database_id) = self.content.get_database_context().map(|(id, _)| id) {
            self.content
                .enter_schema_tables(database_id, schema_name.clone());
            self.status_bar
                .set_message(format!("Viewing tables in '{}'", schema_name));
            let _ = self.action_tx.send(AppAction::LoadData(DataRequest::Tables(
                database_id,
                schema_name,
            )));
        }
    }

    fn handle_drill_down_table(&mut self, table_id: u32, table_name: String) {
        if let Some(database_id) = self.content.get_schema_context().map(|(id, _)| id) {
            self.content
                .enter_table_preview(database_id, table_id, table_name.clone());
            self.status_bar
                .set_message(format!("Loading preview for '{}'...", table_name));
            let _ = self
                .action_tx
                .send(AppAction::LoadData(DataRequest::TablePreview(
                    database_id,
                    table_id,
                )));
        }
    }

    fn handle_schemas_loaded(&mut self, schemas: Vec<String>) {
        self.data.schemas = LoadState::Loaded(schemas.clone());
        self.content.update_schemas(&self.data.schemas);
        self.status_bar
            .set_message(format!("Loaded {} schemas", schemas.len()));
    }

    fn handle_tables_loaded(&mut self, tables: Vec<mbr_core::api::models::TableInfo>) {
        self.data.tables = LoadState::Loaded(tables.clone());
        self.content.update_tables(&self.data.tables);
        self.status_bar
            .set_message(format!("Loaded {} tables", tables.len()));
    }

    fn handle_table_preview_loaded(&mut self, data: QueryResultData) {
        self.data.query_result = Some(data.clone());
        self.content.set_table_preview_data(data.clone());
        self.status_bar
            .set_message(format!("Preview: {} rows loaded", data.rows.len()));
    }
}
