//! Key event handling for ContentPanel.
//!
//! Implements the Component trait's handle_key method with view-specific
//! key bindings for navigation, search, sort, filter, and modal interactions.

use crossterm::event::{KeyCode, KeyEvent};

use super::ContentPanel;
use super::types::{ContentView, InputMode};

impl ContentPanel {
    /// Handle key events for the content panel.
    /// Returns true if the key was handled, false otherwise.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        // Search mode input handling (takes priority in Questions view)
        if self.input_mode == InputMode::Search {
            return self.handle_search_mode_key(key);
        }

        // Delegate to view-specific handlers
        match &self.view {
            ContentView::Questions => self.handle_questions_key(key),
            ContentView::Collections => self.handle_collections_key(key),
            ContentView::Databases => self.handle_databases_key(key),
            ContentView::QueryResult => self.handle_query_result_key(key),
            ContentView::CollectionQuestions { .. } => self.handle_collection_questions_key(key),
            ContentView::DatabaseSchemas { .. } => self.handle_database_schemas_key(key),
            ContentView::SchemaTables { .. } => self.handle_schema_tables_key(key),
            ContentView::TablePreview { .. } => self.handle_table_preview_key(key),
            ContentView::Welcome => self.handle_welcome_key(key),
        }
    }

    /// Handle keys in search mode.
    fn handle_search_mode_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(c) => {
                self.handle_search_input(c);
                true
            }
            KeyCode::Backspace => {
                self.handle_search_backspace();
                true
            }
            // Enter and Esc are handled by App (to send actions)
            _ => false,
        }
    }

    /// Handle keys in Questions view.
    fn handle_questions_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
                true
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.select_first();
                true
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.select_last();
                true
            }
            KeyCode::Char('/') => {
                self.enter_search_mode();
                true
            }
            _ => false,
        }
    }

    /// Handle keys in Collections view.
    fn handle_collections_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_collections_previous();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_collections_next();
                true
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.select_collections_first();
                true
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.select_collections_last();
                true
            }
            _ => false,
        }
    }

    /// Handle keys in Databases view.
    fn handle_databases_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_databases_previous();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_databases_next();
                true
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.select_databases_first();
                true
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.select_databases_last();
                true
            }
            _ => false,
        }
    }

    /// Handle keys in CollectionQuestions view.
    fn handle_collection_questions_key(&mut self, key: KeyEvent) -> bool {
        // Same navigation as Questions, Enter/Esc handled by App
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
                true
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.select_first();
                true
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.select_last();
                true
            }
            _ => false,
        }
    }

    /// Handle keys in DatabaseSchemas view.
    fn handle_database_schemas_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_schemas_previous();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_schemas_next();
                true
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.select_schemas_first();
                true
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.select_schemas_last();
                true
            }
            _ => false,
        }
    }

    /// Handle keys in SchemaTables view.
    fn handle_schema_tables_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_tables_previous();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_tables_next();
                true
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.select_tables_first();
                true
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.select_tables_last();
                true
            }
            _ => false,
        }
    }

    /// Handle keys in QueryResult view.
    fn handle_query_result_key(&mut self, key: KeyEvent) -> bool {
        // Modal handlers take priority
        if self.filter_mode_active {
            return self.handle_filter_modal_key(key);
        }
        if self.sort_mode_active {
            return self.handle_sort_modal_key(key);
        }
        if self.result_search_active {
            return self.handle_result_search_key(key);
        }

        // Normal result navigation
        self.handle_result_navigation_key(key)
    }

    /// Handle keys in TablePreview view (same as QueryResult).
    fn handle_table_preview_key(&mut self, key: KeyEvent) -> bool {
        // Modal handlers take priority
        if self.filter_mode_active {
            return self.handle_filter_modal_key(key);
        }
        if self.sort_mode_active {
            return self.handle_sort_modal_key(key);
        }
        if self.result_search_active {
            return self.handle_result_search_key(key);
        }

        // Normal result navigation
        self.handle_result_navigation_key(key)
    }

    /// Handle keys in Welcome view.
    fn handle_welcome_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll.scroll_up();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll.scroll_down();
                true
            }
            KeyCode::PageUp => {
                for _ in 0..self.scroll.visible {
                    self.scroll.scroll_up();
                }
                true
            }
            KeyCode::PageDown => {
                for _ in 0..self.scroll.visible {
                    self.scroll.scroll_down();
                }
                true
            }
            _ => false,
        }
    }

    /// Handle keys in filter modal.
    fn handle_filter_modal_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.filter_modal_up();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.filter_modal_down();
                true
            }
            KeyCode::Enter => {
                if self.filter_modal_step == 0 {
                    // Column selection → text input
                    self.filter_modal_next_step();
                } else {
                    // Text input → apply filter
                    self.apply_filter();
                }
                true
            }
            KeyCode::Esc => {
                if self.filter_modal_step == 1 {
                    // Text input → column selection
                    self.filter_modal_prev_step();
                } else {
                    // Column selection → close modal
                    self.close_filter_modal();
                }
                true
            }
            KeyCode::Char('f') if self.filter_modal_step == 0 => {
                // Close modal if 'f' pressed in column selection
                self.close_filter_modal();
                true
            }
            KeyCode::Backspace => {
                self.filter_modal_delete_char();
                true
            }
            KeyCode::Char(c) if self.filter_modal_step == 1 => {
                self.filter_modal_input_char(c);
                true
            }
            _ => false,
        }
    }

    /// Handle keys in sort modal.
    fn handle_sort_modal_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.sort_modal_up();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.sort_modal_down();
                true
            }
            KeyCode::Enter => {
                self.apply_sort();
                true
            }
            KeyCode::Esc | KeyCode::Char('s') => {
                self.close_sort_modal();
                true
            }
            _ => false,
        }
    }

    /// Handle keys in result search mode.
    fn handle_result_search_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(c) => {
                self.result_search_input_char(c);
                true
            }
            KeyCode::Backspace => {
                self.result_search_delete_char();
                true
            }
            KeyCode::Enter => {
                self.apply_result_search();
                true
            }
            KeyCode::Esc => {
                self.close_result_search();
                true
            }
            _ => false,
        }
    }

    /// Handle result table navigation keys.
    fn handle_result_navigation_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_result_previous();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_result_next();
                true
            }
            // Horizontal scroll with h/l or Left/Right arrows
            KeyCode::Left | KeyCode::Char('h') => {
                self.scroll_left();
                true
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.scroll_right();
                true
            }
            // Pagination: n for next page, p for previous page (matches CLI)
            KeyCode::Char('n') => {
                self.next_page();
                true
            }
            KeyCode::Char('p') => {
                self.prev_page();
                true
            }
            // PageUp/PageDown for scrolling within page (matches CLI)
            KeyCode::PageUp => {
                self.scroll_result_page_up();
                true
            }
            KeyCode::PageDown => {
                self.scroll_result_page_down();
                true
            }
            // First/Last page with g/G
            KeyCode::Home | KeyCode::Char('g') => {
                self.first_page();
                true
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.last_page();
                true
            }
            // Sort: s to open sort modal
            KeyCode::Char('s') => {
                self.open_sort_modal();
                true
            }
            // Filter: f to open filter modal
            KeyCode::Char('f') => {
                self.open_filter_modal();
                true
            }
            // Clear filter: F (shift+f) to clear filter
            KeyCode::Char('F') => {
                self.clear_filter();
                // Reset to first page after clearing filter
                self.result_page = 0;
                self.result_table_state.select(Some(0));
                true
            }
            // Search: / to open search input
            KeyCode::Char('/') => {
                self.open_result_search();
                true
            }
            // Clear search: Shift+S to clear search
            KeyCode::Char('S') => {
                self.clear_result_search();
                // Reset to first page after clearing search
                self.result_page = 0;
                self.result_table_state.select(Some(0));
                true
            }
            // Note: Esc is handled in App for returning to previous view
            _ => false,
        }
    }
}
