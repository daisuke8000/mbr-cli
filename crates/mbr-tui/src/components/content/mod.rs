//! Content panel component.
//!
//! Displays the main content area (query results, question details, etc.).
//!
//! This module is split into submodules for better organization:
//! - `types`: View types, input modes, and data structures
//! - `views`: View rendering functions (Welcome, Questions, Collections, etc.)
//! - `modals`: Modal overlay rendering (Sort, Filter, Search)
//! - `navigation`: Navigation stack and view transitions
//! - `selection`: Selection navigation for each view type
//! - `search`: Search mode handling
//! - `sort`: Sort functionality for query results
//! - `filter`: Filter functionality for query results
//! - `result_search`: All-column search in results
//! - `pagination`: Pagination and scrolling
//! - `key_handler`: Key event handling

mod filter;
mod key_handler;
mod modals;
mod navigation;
mod pagination;
mod result_search;
mod search;
mod selection;
mod sort;
pub mod types;
mod views;

use std::collections::HashSet;

use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect, widgets::TableState};

use mbr_core::api::models::{CollectionItem, Database, Question, TableInfo};

pub use types::{ContentView, InputMode, QueryResultData, SortOrder};

use super::{Component, ScrollState};
use crate::service::LoadState;
use types::DEFAULT_ROWS_PER_PAGE;

/// Content panel showing main content.
pub struct ContentPanel {
    pub(super) view: ContentView,
    pub(super) scroll: ScrollState,
    /// Horizontal scroll offset (column index)
    pub(super) scroll_x: usize,
    /// Questions data for the Questions view
    pub(super) questions: LoadState<Vec<Question>>,
    /// Table state for Questions view (manages selection and scroll)
    pub(super) table_state: TableState,
    /// Collections data for the Collections view
    pub(super) collections: LoadState<Vec<CollectionItem>>,
    /// Table state for Collections view
    pub(super) collections_table_state: TableState,
    /// Databases data for the Databases view
    pub(super) databases: LoadState<Vec<Database>>,
    /// Table state for Databases view
    pub(super) databases_table_state: TableState,
    /// Schemas data for the DatabaseSchemas view
    pub(super) schemas: LoadState<Vec<String>>,
    /// Table state for Schemas view
    pub(super) schemas_table_state: TableState,
    /// Tables data for the SchemaTables view
    pub(super) tables: LoadState<Vec<TableInfo>>,
    /// Table state for Tables view
    pub(super) tables_table_state: TableState,
    /// Query result data for QueryResult view
    pub(super) query_result: Option<QueryResultData>,
    /// Sorted row indices (None = original order, Some = sorted indices)
    /// Using indices instead of copying rows for memory efficiency
    pub(super) sort_indices: Option<Vec<usize>>,
    /// Table state for query result table
    pub(super) result_table_state: TableState,
    /// Current page for query result pagination (0-indexed)
    pub(super) result_page: usize,
    /// Rows per page for query result pagination
    pub(super) rows_per_page: usize,
    /// Current input mode
    pub(super) input_mode: InputMode,
    /// Current search query
    pub(super) search_query: String,
    /// Active search query (used for display after search is executed)
    pub(super) active_search: Option<String>,
    /// Navigation stack for multi-level drill-down (supports 4+ levels)
    /// Context data is now embedded directly in ContentView variants.
    /// Used for: Databases → Schemas → Tables → Preview
    ///           Collections → Questions → QueryResult
    pub(super) navigation_stack: Vec<ContentView>,
    /// Sort order for query results
    pub(super) sort_order: SortOrder,
    /// Column index currently being sorted (None = no sort)
    pub(super) sort_column_index: Option<usize>,
    /// Whether sort column selection modal is active
    pub(super) sort_mode_active: bool,
    /// Selected column index in sort modal
    pub(super) sort_modal_selection: usize,
    // === Filter state ===
    /// Filtered row indices (None = no filter, Some = filtered indices)
    pub(super) filter_indices: Option<Vec<usize>>,
    /// Column index currently being filtered (None = no filter)
    pub(super) filter_column_index: Option<usize>,
    /// Filter text (case-insensitive contains match)
    pub(super) filter_text: String,
    /// Whether filter modal is active
    pub(super) filter_mode_active: bool,
    /// Current step in filter modal (0 = column selection, 1 = text input)
    pub(super) filter_modal_step: usize,
    /// Selected column index in filter modal
    pub(super) filter_modal_selection: usize,
    // === Result Search state (all-column search) ===
    /// Whether result search mode is active
    pub(super) result_search_active: bool,
    /// Search text for result search (all-column, case-insensitive)
    pub(super) result_search_text: String,
    /// Searched row indices (None = no search, Some = matched indices)
    pub(super) result_search_indices: Option<Vec<usize>>,
    // === Multi-select state (for result views) ===
    /// Selected row indices (original row indices, not display indices)
    /// Uses row index as identifier since QueryResultData doesn't have row IDs
    pub(super) selected_rows: HashSet<usize>,
    /// Anchor position for range selection (Shift+Arrow)
    /// Stores the starting row index when range selection begins
    pub(super) selection_anchor: Option<usize>,
}

impl Default for ContentPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentPanel {
    /// Create a new content panel.
    pub fn new() -> Self {
        Self {
            view: ContentView::Welcome,
            scroll: ScrollState::default(),
            scroll_x: 0,
            questions: LoadState::default(),
            table_state: TableState::default(),
            collections: LoadState::default(),
            collections_table_state: TableState::default(),
            databases: LoadState::default(),
            databases_table_state: TableState::default(),
            schemas: LoadState::default(),
            schemas_table_state: TableState::default(),
            tables: LoadState::default(),
            tables_table_state: TableState::default(),
            query_result: None,
            sort_indices: None,
            result_table_state: TableState::default(),
            result_page: 0,
            rows_per_page: DEFAULT_ROWS_PER_PAGE,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            active_search: None,
            navigation_stack: Vec::new(),
            sort_order: SortOrder::None,
            sort_column_index: None,
            sort_mode_active: false,
            sort_modal_selection: 0,
            filter_indices: None,
            filter_column_index: None,
            filter_text: String::new(),
            filter_mode_active: false,
            filter_modal_step: 0,
            filter_modal_selection: 0,
            result_search_active: false,
            result_search_text: String::new(),
            result_search_indices: None,
            selected_rows: HashSet::new(),
            selection_anchor: None,
        }
    }

    /// Set the current view (used for tab switching).
    /// Clears navigation stack since tab changes reset the navigation context.
    pub fn set_view(&mut self, view: ContentView) {
        self.view = view;
        self.scroll = ScrollState::default();
        self.scroll_x = 0;
        self.table_state = TableState::default();
        // Clear navigation stack on tab switch
        self.clear_navigation_stack();
    }

    /// Get the current view (cloned since ContentView now contains data).
    pub fn current_view(&self) -> ContentView {
        self.view.clone()
    }

    /// Check if the current view is a specific type (ignoring embedded data).
    #[allow(dead_code)] // Utility method for future use
    pub fn is_view(&self, view_type: &ContentView) -> bool {
        std::mem::discriminant(&self.view) == std::mem::discriminant(view_type)
    }

    /// Check if current view is Questions or CollectionQuestions.
    pub fn is_questions_view(&self) -> bool {
        matches!(
            self.view,
            ContentView::Questions | ContentView::CollectionQuestions { .. }
        )
    }

    /// Check if current view is CollectionQuestions.
    pub fn is_collection_questions_view(&self) -> bool {
        matches!(self.view, ContentView::CollectionQuestions { .. })
    }

    /// Check if current view is DatabaseSchemas.
    pub fn is_database_schemas_view(&self) -> bool {
        matches!(self.view, ContentView::DatabaseSchemas { .. })
    }

    /// Check if current view is SchemaTables.
    pub fn is_schema_tables_view(&self) -> bool {
        matches!(self.view, ContentView::SchemaTables { .. })
    }

    /// Check if current view is TablePreview.
    pub fn is_table_preview_view(&self) -> bool {
        matches!(self.view, ContentView::TablePreview { .. })
    }

    /// Check if current view is QueryResult or TablePreview (both show query results).
    pub fn is_result_view(&self) -> bool {
        matches!(
            self.view,
            ContentView::QueryResult | ContentView::TablePreview { .. }
        )
    }

    // === Multi-select methods ===

    /// Toggle selection for a row (by original row index).
    pub fn toggle_row_selection(&mut self, row_index: usize) {
        if self.selected_rows.contains(&row_index) {
            self.selected_rows.remove(&row_index);
        } else {
            self.selected_rows.insert(row_index);
        }
    }

    /// Check if a row is selected.
    pub fn is_row_selected(&self, row_index: usize) -> bool {
        self.selected_rows.contains(&row_index)
    }

    /// Clear all selections.
    pub fn clear_selection(&mut self) {
        self.selected_rows.clear();
        self.selection_anchor = None;
    }

    /// Get the number of selected rows.
    pub fn selected_count(&self) -> usize {
        self.selected_rows.len()
    }

    /// Check if any rows are selected.
    pub fn has_selection(&self) -> bool {
        !self.selected_rows.is_empty()
    }

    /// Get all selected row indices (sorted).
    pub fn get_selected_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self.selected_rows.iter().copied().collect();
        indices.sort_unstable();
        indices
    }

    /// Select all visible rows in the current result.
    pub fn select_all_rows(&mut self) {
        if let Some(ref result) = self.query_result {
            let total = self.get_visible_row_count();
            for display_idx in 0..total {
                if let Some(original_idx) = self.get_original_row_index(display_idx) {
                    self.selected_rows.insert(original_idx);
                }
            }
        }
    }

    /// Extend selection from anchor to current cursor position.
    /// Used for Shift+Arrow range selection.
    pub fn extend_selection_to(&mut self, target_display_idx: usize) {
        // Set anchor if not already set
        if self.selection_anchor.is_none() {
            if let Some(current) = self.result_table_state.selected() {
                self.selection_anchor = Some(current);
            }
        }

        if let Some(anchor) = self.selection_anchor {
            let (start, end) = if anchor <= target_display_idx {
                (anchor, target_display_idx)
            } else {
                (target_display_idx, anchor)
            };

            // Select all rows in the range
            for display_idx in start..=end {
                if let Some(original_idx) = self.get_original_row_index(display_idx) {
                    self.selected_rows.insert(original_idx);
                }
            }
        }
    }

    /// Get the original row index from a display index.
    /// Accounts for sort, filter, and search transformations.
    fn get_original_row_index(&self, display_idx: usize) -> Option<usize> {
        // Priority: result_search > filter > sort > original
        if let Some(ref search_indices) = self.result_search_indices {
            search_indices.get(display_idx).copied()
        } else if let Some(ref filter_indices) = self.filter_indices {
            filter_indices.get(display_idx).copied()
        } else if let Some(ref sort_indices) = self.sort_indices {
            sort_indices.get(display_idx).copied()
        } else {
            Some(display_idx)
        }
    }

    /// Get the total number of visible rows (after filter/search).
    fn get_visible_row_count(&self) -> usize {
        if let Some(ref search_indices) = self.result_search_indices {
            search_indices.len()
        } else if let Some(ref filter_indices) = self.filter_indices {
            filter_indices.len()
        } else if let Some(ref sort_indices) = self.sort_indices {
            sort_indices.len()
        } else if let Some(ref result) = self.query_result {
            result.rows.len()
        } else {
            0
        }
    }

    /// Get the current cursor's original row index.
    pub fn get_current_row_index(&self) -> Option<usize> {
        let display_idx = self.result_table_state.selected()?;
        // Convert page-local index to global display index
        let global_display_idx = self.result_page * self.rows_per_page + display_idx;
        self.get_original_row_index(global_display_idx)
    }

    /// Get selected records as (columns, values) pairs.
    pub fn get_selected_records(&self) -> Vec<(Vec<String>, Vec<String>)> {
        let result = match &self.query_result {
            Some(r) => r,
            None => return vec![],
        };

        let indices = self.get_selected_indices();
        indices
            .iter()
            .filter_map(|&idx| {
                result.rows.get(idx).map(|row| {
                    (result.columns.clone(), row.clone())
                })
            })
            .collect()
    }
}

impl Component for ContentPanel {
    fn draw(&mut self, frame: &mut Frame, area: Rect, focused: bool) {
        // Table-based views render directly (uses stateful Table widget)
        match self.view {
            ContentView::Questions => {
                self.render_questions(area, frame, focused);
                return;
            }
            ContentView::Collections => {
                self.render_collections(area, frame, focused);
                return;
            }
            ContentView::Databases => {
                self.render_databases(area, frame, focused);
                return;
            }
            ContentView::QueryResult => {
                self.render_query_result(area, frame, focused);
                return;
            }
            ContentView::CollectionQuestions { .. } => {
                self.render_collection_questions(area, frame, focused);
                return;
            }
            ContentView::DatabaseSchemas { .. } => {
                self.render_database_schemas(area, frame, focused);
                return;
            }
            ContentView::SchemaTables { .. } => {
                self.render_schema_tables(area, frame, focused);
                return;
            }
            ContentView::TablePreview { .. } => {
                self.render_table_preview(area, frame, focused);
                return;
            }
            ContentView::Welcome => {}
        }

        // Welcome view returns Paragraph widget
        let widget = self.render_welcome(area, focused);
        frame.render_widget(widget, area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        self.handle_key_event(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Navigation Stack Tests ===

    #[test]
    fn test_push_pop_view() {
        let mut panel = ContentPanel::new();
        assert_eq!(panel.current_view(), ContentView::Welcome);
        assert_eq!(panel.navigation_depth(), 0);

        // Push to Questions view
        panel.push_view(ContentView::Questions);
        assert_eq!(panel.current_view(), ContentView::Questions);
        assert_eq!(panel.navigation_depth(), 1);

        // Push to QueryResult view
        panel.push_view(ContentView::QueryResult);
        assert_eq!(panel.current_view(), ContentView::QueryResult);
        assert_eq!(panel.navigation_depth(), 2);

        // Pop back to Questions
        let popped = panel.pop_view();
        assert_eq!(popped, Some(ContentView::Questions));
        assert_eq!(panel.current_view(), ContentView::Questions);
        assert_eq!(panel.navigation_depth(), 1);

        // Pop back to Welcome
        let popped = panel.pop_view();
        assert_eq!(popped, Some(ContentView::Welcome));
        assert_eq!(panel.current_view(), ContentView::Welcome);
        assert_eq!(panel.navigation_depth(), 0);

        // Pop on empty stack returns None
        let popped = panel.pop_view();
        assert_eq!(popped, None);
        assert_eq!(panel.current_view(), ContentView::Welcome);
    }

    #[test]
    fn test_navigation_stack_four_level_drill_down() {
        let mut panel = ContentPanel::new();

        // Simulate: Databases → Schemas → Tables → Preview (4 levels)
        panel.set_view(ContentView::Databases);
        panel.push_view(ContentView::DatabaseSchemas {
            db_id: 1,
            db_name: "TestDB".to_string(),
        });
        panel.push_view(ContentView::SchemaTables {
            db_id: 1,
            schema_name: "public".to_string(),
        });
        panel.push_view(ContentView::TablePreview {
            db_id: 1,
            table_id: 10,
            table_name: "users".to_string(),
        });

        assert!(panel.is_table_preview_view());
        assert_eq!(panel.navigation_depth(), 3);

        // Go back step by step
        panel.pop_view();
        assert!(panel.is_schema_tables_view());

        panel.pop_view();
        assert!(panel.is_database_schemas_view());

        panel.pop_view();
        assert_eq!(panel.current_view(), ContentView::Databases);
    }

    #[test]
    fn test_clear_navigation_stack() {
        let mut panel = ContentPanel::new();

        // Build up a navigation stack
        panel.push_view(ContentView::Questions);
        panel.push_view(ContentView::QueryResult);
        assert_eq!(panel.navigation_depth(), 2);

        // Clear stack
        panel.clear_navigation_stack();
        assert_eq!(panel.navigation_depth(), 0);
    }

    #[test]
    fn test_set_view_clears_navigation_stack() {
        let mut panel = ContentPanel::new();

        // Build up navigation
        panel.push_view(ContentView::Questions);
        panel.push_view(ContentView::QueryResult);
        assert_eq!(panel.navigation_depth(), 2);

        // Tab switch should clear stack
        panel.set_view(ContentView::Collections);
        assert_eq!(panel.current_view(), ContentView::Collections);
        assert_eq!(panel.navigation_depth(), 0);
    }

    // === Search Mode Tests ===

    #[test]
    fn test_search_mode_toggle() {
        let mut panel = ContentPanel::new();
        assert_eq!(panel.input_mode(), InputMode::Normal);

        // Enter search mode
        panel.enter_search_mode();
        assert_eq!(panel.input_mode(), InputMode::Search);

        // Exit search mode
        panel.exit_search_mode();
        assert_eq!(panel.input_mode(), InputMode::Normal);
    }

    #[test]
    fn test_search_query_input() {
        let mut panel = ContentPanel::new();
        panel.enter_search_mode();

        // Type characters
        panel.handle_search_input('h');
        panel.handle_search_input('e');
        panel.handle_search_input('l');
        panel.handle_search_input('l');
        panel.handle_search_input('o');

        assert_eq!(panel.get_search_query(), "hello");

        // Backspace
        panel.handle_search_backspace();
        assert_eq!(panel.get_search_query(), "hell");
    }

    #[test]
    fn test_execute_search() {
        let mut panel = ContentPanel::new();
        panel.enter_search_mode();

        panel.handle_search_input('t');
        panel.handle_search_input('e');
        panel.handle_search_input('s');
        panel.handle_search_input('t');

        // Execute search
        let result = panel.execute_search();
        assert_eq!(result, Some("test".to_string()));
        assert_eq!(panel.input_mode(), InputMode::Normal);
        assert_eq!(panel.get_active_search(), Some("test"));
    }

    #[test]
    fn test_execute_empty_search_returns_none() {
        let mut panel = ContentPanel::new();
        panel.enter_search_mode();

        // Execute without typing anything
        let result = panel.execute_search();
        assert_eq!(result, None);
    }

    #[test]
    fn test_clear_search() {
        let mut panel = ContentPanel::new();
        panel.enter_search_mode();
        panel.handle_search_input('t');
        panel.handle_search_input('e');
        panel.handle_search_input('s');
        panel.handle_search_input('t');
        panel.execute_search();

        assert_eq!(panel.get_active_search(), Some("test"));

        panel.clear_search();
        assert_eq!(panel.get_active_search(), None);
        assert_eq!(panel.get_search_query(), "");
    }

    // === Modal State Tests ===

    #[test]
    fn test_sort_mode_toggle() {
        let mut panel = ContentPanel::new();
        assert!(!panel.is_sort_mode_active());

        // Sort modal requires query_result with columns
        let data = QueryResultData {
            question_id: 1,
            question_name: "Test".to_string(),
            columns: vec!["Col1".to_string(), "Col2".to_string()],
            rows: vec![vec!["a".to_string(), "b".to_string()]],
        };
        panel.set_query_result(data);

        panel.open_sort_modal();
        assert!(panel.is_sort_mode_active());

        panel.close_sort_modal();
        assert!(!panel.is_sort_mode_active());
    }

    #[test]
    fn test_filter_mode_toggle() {
        let mut panel = ContentPanel::new();
        assert!(!panel.is_filter_mode_active());

        // Filter modal requires query_result with columns
        let data = QueryResultData {
            question_id: 1,
            question_name: "Test".to_string(),
            columns: vec!["Col1".to_string(), "Col2".to_string()],
            rows: vec![vec!["a".to_string(), "b".to_string()]],
        };
        panel.set_query_result(data);

        panel.open_filter_modal();
        assert!(panel.is_filter_mode_active());

        panel.close_filter_modal();
        assert!(!panel.is_filter_mode_active());
    }

    #[test]
    fn test_result_search_toggle() {
        let mut panel = ContentPanel::new();
        assert!(!panel.is_result_search_active());

        panel.open_result_search();
        assert!(panel.is_result_search_active());

        panel.close_result_search();
        assert!(!panel.is_result_search_active());
    }

    // === View Transition Tests ===

    #[test]
    fn test_content_view_default() {
        let panel = ContentPanel::new();
        assert_eq!(panel.current_view(), ContentView::Welcome);
    }

    #[test]
    fn test_set_view_changes_current_view() {
        let mut panel = ContentPanel::new();

        panel.set_view(ContentView::Questions);
        assert_eq!(panel.current_view(), ContentView::Questions);

        panel.set_view(ContentView::Collections);
        assert_eq!(panel.current_view(), ContentView::Collections);

        panel.set_view(ContentView::Databases);
        assert_eq!(panel.current_view(), ContentView::Databases);
    }

    // === Selection Navigation Tests ===

    #[test]
    fn test_select_navigation_with_loaded_questions() {
        let mut panel = ContentPanel::new();
        panel.set_view(ContentView::Questions);

        // Load some questions
        let questions = vec![
            Question {
                id: 1,
                name: "Q1".to_string(),
                description: None,
                collection_id: None,
                collection: None,
            },
            Question {
                id: 2,
                name: "Q2".to_string(),
                description: None,
                collection_id: None,
                collection: None,
            },
            Question {
                id: 3,
                name: "Q3".to_string(),
                description: None,
                collection_id: None,
                collection: None,
            },
        ];
        panel.update_questions(&LoadState::Loaded(questions));

        // First item should be auto-selected
        assert_eq!(panel.table_state.selected(), Some(0));

        // Navigate down
        panel.select_next();
        assert_eq!(panel.table_state.selected(), Some(1));

        panel.select_next();
        assert_eq!(panel.table_state.selected(), Some(2));

        // Should not go beyond last item
        panel.select_next();
        assert_eq!(panel.table_state.selected(), Some(2));

        // Navigate up
        panel.select_previous();
        assert_eq!(panel.table_state.selected(), Some(1));

        // Jump to first
        panel.select_first();
        assert_eq!(panel.table_state.selected(), Some(0));

        // Jump to last
        panel.select_last();
        assert_eq!(panel.table_state.selected(), Some(2));
    }

    #[test]
    fn test_select_navigation_with_empty_list() {
        let mut panel = ContentPanel::new();
        panel.set_view(ContentView::Questions);

        // Load empty list
        panel.update_questions(&LoadState::Loaded(vec![]));

        // Navigation should not panic
        panel.select_next();
        panel.select_previous();
        panel.select_first();
        panel.select_last();
    }

    // === Collection Questions Context Tests ===

    #[test]
    fn test_enter_collection_questions_sets_context() {
        let mut panel = ContentPanel::new();
        panel.set_view(ContentView::Collections);

        panel.enter_collection_questions(42, "Test Collection".to_string());

        assert!(panel.is_collection_questions_view());
        assert_eq!(
            panel.get_collection_context(),
            Some((42_u32, "Test Collection".to_string()))
        );
    }

    #[test]
    fn test_exit_collection_questions_clears_context() {
        let mut panel = ContentPanel::new();
        panel.set_view(ContentView::Collections);
        panel.enter_collection_questions(42, "Test Collection".to_string());

        panel.exit_collection_questions();

        assert_eq!(panel.current_view(), ContentView::Collections);
        assert_eq!(panel.get_collection_context(), None);
    }

    // === Database Drill-down Context Tests ===

    #[test]
    fn test_database_schema_drill_down() {
        let mut panel = ContentPanel::new();
        panel.set_view(ContentView::Databases);

        // Enter database schemas
        panel.enter_database_schemas(1, "TestDB".to_string());
        assert!(panel.is_database_schemas_view());
        assert_eq!(
            panel.get_database_context(),
            Some((1_u32, "TestDB".to_string()))
        );

        // Exit back to databases
        panel.exit_database_schemas();
        assert_eq!(panel.current_view(), ContentView::Databases);
        assert_eq!(panel.get_database_context(), None);
    }

    #[test]
    fn test_schema_tables_drill_down() {
        let mut panel = ContentPanel::new();
        panel.set_view(ContentView::Databases);
        panel.enter_database_schemas(1, "TestDB".to_string());

        // Enter schema tables
        panel.enter_schema_tables(1, "public".to_string());
        assert!(panel.is_schema_tables_view());
        assert_eq!(
            panel.get_schema_context(),
            Some((1_u32, "public".to_string()))
        );

        // Exit back to schemas
        panel.exit_schema_tables();
        assert!(panel.is_database_schemas_view());
        assert_eq!(panel.get_schema_context(), None);
    }

    // === Query Result Tests ===

    #[test]
    fn test_set_query_result() {
        let mut panel = ContentPanel::new();
        panel.set_view(ContentView::Questions);

        let result_data = QueryResultData {
            question_id: 1,
            question_name: "Test Query".to_string(),
            columns: vec!["ID".to_string(), "Name".to_string()],
            rows: vec![
                vec!["1".to_string(), "Alice".to_string()],
                vec!["2".to_string(), "Bob".to_string()],
            ],
        };

        panel.set_query_result(result_data.clone());

        assert_eq!(panel.current_view(), ContentView::QueryResult);
        assert!(panel.query_result.is_some());
    }

    #[test]
    fn test_back_to_questions_clears_result() {
        let mut panel = ContentPanel::new();
        panel.set_view(ContentView::Questions);

        let result_data = QueryResultData {
            question_id: 1,
            question_name: "Test Query".to_string(),
            columns: vec!["ID".to_string()],
            rows: vec![vec!["1".to_string()]],
        };

        panel.set_query_result(result_data);
        assert!(panel.query_result.is_some());

        panel.back_to_questions();
        assert!(panel.query_result.is_none());
        assert_eq!(panel.current_view(), ContentView::Questions);
    }

    // === Load State Tests ===

    #[test]
    fn test_update_questions_auto_selects_first() {
        let mut panel = ContentPanel::new();

        let questions = vec![Question {
            id: 1,
            name: "Q1".to_string(),
            description: None,
            collection_id: None,
            collection: None,
        }];

        panel.update_questions(&LoadState::Loaded(questions));
        assert_eq!(panel.table_state.selected(), Some(0));
    }

    #[test]
    fn test_update_collections_auto_selects_first() {
        let mut panel = ContentPanel::new();

        let collections = vec![CollectionItem {
            id: Some(1),
            name: "C1".to_string(),
            description: None,
            location: None,
            personal_owner_id: None,
            archived: false,
        }];

        panel.update_collections(&LoadState::Loaded(collections));
        assert_eq!(panel.collections_table_state.selected(), Some(0));
    }

    #[test]
    fn test_update_databases_auto_selects_first() {
        let mut panel = ContentPanel::new();

        let databases = vec![Database {
            id: 1,
            name: "DB1".to_string(),
            engine: Some("postgres".to_string()),
            description: None,
            is_sample: false,
            is_saved_questions: false,
        }];

        panel.update_databases(&LoadState::Loaded(databases));
        assert_eq!(panel.databases_table_state.selected(), Some(0));
    }
}
