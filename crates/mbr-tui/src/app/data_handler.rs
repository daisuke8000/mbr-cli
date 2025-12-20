//! Data loading request handling.
//!
//! Handles DataRequest events by spawning async tasks to fetch data from the API.

use std::sync::Arc;

use crate::action::{AppAction, DataRequest};
use crate::components::QueryResultData;
use crate::service::LoadState;

use super::App;

impl App {
    /// Handle data loading request with background task spawning.
    pub(super) fn handle_data_request(&mut self, request: DataRequest) {
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
                self.load_questions(service, tx);
            }
            DataRequest::SearchQuestions(query) => {
                self.search_questions(service, tx, query);
            }
            DataRequest::FilterQuestionsByCollection(collection_id) => {
                self.load_collection_questions(service, tx, collection_id);
            }
            DataRequest::Collections => {
                self.load_collections(service, tx);
            }
            DataRequest::Databases => {
                self.load_databases(service, tx);
            }
            DataRequest::QuestionDetails(id) => {
                self.status_bar
                    .set_message(format!("Loading question #{}...", id));
            }
            DataRequest::Execute(id) => {
                self.status_bar
                    .set_message(format!("Executing query #{}...", id));
            }
            DataRequest::Schemas(database_id) => {
                self.load_schemas(service, tx, database_id);
            }
            DataRequest::Tables(database_id, schema_name) => {
                self.load_tables(service, tx, database_id, schema_name);
            }
            DataRequest::TablePreview(database_id, table_id) => {
                self.load_table_preview(service, tx, database_id, table_id);
            }
        }
    }

    fn load_questions(
        &mut self,
        service: Arc<crate::service::ServiceClient>,
        tx: tokio::sync::mpsc::UnboundedSender<AppAction>,
    ) {
        if matches!(self.data.questions, LoadState::Loading) {
            return;
        }

        self.data.questions = LoadState::Loading;
        self.content.update_questions(&self.data.questions);
        self.status_bar.set_message("Loading questions...");

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

    fn search_questions(
        &mut self,
        service: Arc<crate::service::ServiceClient>,
        tx: tokio::sync::mpsc::UnboundedSender<AppAction>,
        query: String,
    ) {
        if matches!(self.data.questions, LoadState::Loading) {
            return;
        }

        self.data.questions = LoadState::Loading;
        self.content.update_questions(&self.data.questions);
        self.status_bar
            .set_message(format!("Searching for '{}'...", query));

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

    fn load_collection_questions(
        &mut self,
        service: Arc<crate::service::ServiceClient>,
        tx: tokio::sync::mpsc::UnboundedSender<AppAction>,
        collection_id: u32,
    ) {
        if matches!(self.data.questions, LoadState::Loading) {
            return;
        }

        self.data.questions = LoadState::Loading;
        self.content.update_questions(&self.data.questions);

        let collection_str = collection_id.to_string();

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

    fn load_collections(
        &mut self,
        service: Arc<crate::service::ServiceClient>,
        tx: tokio::sync::mpsc::UnboundedSender<AppAction>,
    ) {
        if matches!(self.data.collections, LoadState::Loading) {
            return;
        }

        self.data.collections = LoadState::Loading;
        self.content.update_collections(&self.data.collections);
        self.status_bar.set_message("Loading collections...");

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

    fn load_databases(
        &mut self,
        service: Arc<crate::service::ServiceClient>,
        tx: tokio::sync::mpsc::UnboundedSender<AppAction>,
    ) {
        if matches!(self.data.databases, LoadState::Loading) {
            return;
        }

        self.data.databases = LoadState::Loading;
        self.content.update_databases(&self.data.databases);
        self.status_bar.set_message("Loading databases...");

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

    fn load_schemas(
        &mut self,
        service: Arc<crate::service::ServiceClient>,
        tx: tokio::sync::mpsc::UnboundedSender<AppAction>,
        database_id: u32,
    ) {
        if matches!(self.data.schemas, LoadState::Loading) {
            return;
        }

        self.data.schemas = LoadState::Loading;
        self.content.update_schemas(&self.data.schemas);
        self.status_bar.set_message("Loading schemas...");

        tokio::spawn(async move {
            match service.fetch_schemas(database_id).await {
                Ok(schemas) => {
                    let _ = tx.send(AppAction::SchemasLoaded(schemas));
                }
                Err(e) => {
                    let _ = tx.send(AppAction::LoadFailed(DataRequest::Schemas(database_id), e));
                }
            }
        });
    }

    fn load_tables(
        &mut self,
        service: Arc<crate::service::ServiceClient>,
        tx: tokio::sync::mpsc::UnboundedSender<AppAction>,
        database_id: u32,
        schema_name: String,
    ) {
        if matches!(self.data.tables, LoadState::Loading) {
            return;
        }

        self.data.tables = LoadState::Loading;
        self.content.update_tables(&self.data.tables);
        self.status_bar
            .set_message(format!("Loading tables in '{}'...", schema_name));

        let schema = schema_name.clone();

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

    fn load_table_preview(
        &mut self,
        service: Arc<crate::service::ServiceClient>,
        tx: tokio::sync::mpsc::UnboundedSender<AppAction>,
        database_id: u32,
        table_id: u32,
    ) {
        self.status_bar.set_message("Loading table preview...");

        tokio::spawn(async move {
            match service.preview_table(database_id, table_id, 100).await {
                Ok(result) => {
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

    /// Execute a question query.
    pub(super) fn execute_question(&mut self, id: u32) {
        let service = match &self.service {
            Some(s) => Arc::clone(s),
            None => {
                self.status_bar
                    .set_message("Error: Not connected to Metabase");
                return;
            }
        };

        self.current_request_id = self.current_request_id.wrapping_add(1);
        let request_id = self.current_request_id;

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
}
