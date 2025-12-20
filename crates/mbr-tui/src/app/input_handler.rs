//! Keyboard input handling for the application.
//!
//! Processes key events and delegates to appropriate handlers based on
//! current application state (overlays, modals, content panels).

use crossterm::event::{KeyCode, KeyModifiers};

use crate::action::{AppAction, DataRequest};
use crate::components::clipboard::{
    CopyFormat, copy_to_clipboard, format_record_csv, format_record_json, format_record_tsv,
};
use crate::components::{ContentView, CopyMenu, InputMode, RecordDetailOverlay};
use crate::service::LoadState;

use super::App;

impl App {
    /// Handle keyboard input with delegated responsibility.
    pub(super) fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        // 1. Overlay handling (highest priority)
        if self.handle_overlay_keys(code) {
            return;
        }

        // 2. Search mode handling
        if self.handle_search_mode_keys(code, modifiers) {
            return;
        }

        // 3. Global keybindings
        if self.handle_global_keys(code, modifiers) {
            return;
        }

        // 4. Enter key for content actions
        if code == KeyCode::Enter && self.handle_enter_key() {
            return;
        }

        // 5. Delegate remaining keys to content panel
        self.content
            .handle_key_event(crossterm::event::KeyEvent::new(code, modifiers));
    }

    /// Handle keyboard input when overlay is active (CopyMenu, RecordDetail, Help).
    /// Returns true if the key was handled.
    pub(super) fn handle_overlay_keys(&mut self, code: KeyCode) -> bool {
        // Copy menu takes highest priority when shown
        if self.show_copy_menu {
            match code {
                KeyCode::Esc => {
                    self.close_copy_menu();
                }
                // Arrow keys for navigation (j/k are used for format selection)
                KeyCode::Up => {
                    if let Some(ref mut menu) = self.copy_menu {
                        menu.select_up();
                    }
                }
                KeyCode::Down => {
                    if let Some(ref mut menu) = self.copy_menu {
                        menu.select_down();
                    }
                }
                // Header toggle
                KeyCode::Char('h') => {
                    if let Some(ref mut menu) = self.copy_menu {
                        menu.toggle_header();
                    }
                }
                // Direct format selection → immediate copy
                KeyCode::Char('j') => {
                    self.execute_copy(CopyFormat::Json);
                }
                KeyCode::Char('c') => {
                    self.execute_copy(CopyFormat::Csv);
                }
                KeyCode::Char('t') => {
                    self.execute_copy(CopyFormat::Tsv);
                }
                // Enter copies with selected format
                KeyCode::Enter => {
                    if let Some(ref menu) = self.copy_menu {
                        let format = menu.selected_format();
                        self.execute_copy(format);
                    }
                }
                _ => {}
            }
            return true;
        }

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
                KeyCode::Char('c') => {
                    if let Some(ref detail) = self.record_detail {
                        if let Some(value) = detail.selected_value() {
                            match copy_to_clipboard(value) {
                                Ok(()) => {
                                    self.status_bar.set_message("Copied to clipboard");
                                }
                                Err(e) => {
                                    self.status_bar.set_message(format!("Copy failed: {}", e));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
            return true;
        }

        // Help overlay takes priority when shown
        if self.show_help {
            match code {
                KeyCode::Char('?') | KeyCode::Esc => {
                    self.show_help = false;
                }
                _ => {}
            }
            return true;
        }

        false
    }

    /// Handle keyboard input in search mode.
    /// Returns true if the key was handled.
    pub(super) fn handle_search_mode_keys(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
    ) -> bool {
        if self.content.input_mode() != InputMode::Search {
            return false;
        }

        match code {
            KeyCode::Enter => {
                if let Some(query) = self.content.execute_search() {
                    let _ = self
                        .action_tx
                        .send(AppAction::LoadData(DataRequest::SearchQuestions(query)));
                } else {
                    let _ = self
                        .action_tx
                        .send(AppAction::LoadData(DataRequest::Questions));
                }
            }
            KeyCode::Esc => {
                self.content.exit_search_mode();
            }
            _ => {
                self.content
                    .handle_key_event(crossterm::event::KeyEvent::new(code, modifiers));
            }
        }
        true
    }

    /// Handle global keybindings (quit, help, tab switch, refresh).
    /// Returns true if the key was handled.
    pub(super) fn handle_global_keys(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        match code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.should_quit = true;
                true
            }
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                true
            }
            KeyCode::Esc => self.handle_escape_key(),
            KeyCode::Char('?') if !self.is_modal_active() => {
                self.show_help = true;
                true
            }
            // Tab switching with number keys 1/2/3
            KeyCode::Char('1') if !self.is_modal_active() => {
                self.switch_to_tab(crate::components::ActiveTab::Questions);
                true
            }
            KeyCode::Char('2') if !self.is_modal_active() => {
                self.switch_to_tab(crate::components::ActiveTab::Collections);
                true
            }
            KeyCode::Char('3') if !self.is_modal_active() => {
                self.switch_to_tab(crate::components::ActiveTab::Databases);
                true
            }
            // Tab cycling with Tab/Shift+Tab
            KeyCode::Tab if !self.is_modal_active() => {
                let new_tab = if modifiers.contains(KeyModifiers::SHIFT) {
                    self.active_tab.previous()
                } else {
                    self.active_tab.next()
                };
                self.switch_to_tab(new_tab);
                true
            }
            KeyCode::BackTab if !self.is_modal_active() => {
                self.switch_to_tab(self.active_tab.previous());
                true
            }
            // Refresh data with 'r'
            KeyCode::Char('r') if !self.is_modal_active() => {
                self.handle_refresh();
                true
            }
            // Copy record in result view with 'c'
            KeyCode::Char('c') if self.content.is_result_view() && !self.is_modal_active() => {
                if let Some((columns, values)) = self.content.get_selected_record() {
                    self.copy_menu = Some(CopyMenu::new(columns, values));
                    self.show_copy_menu = true;
                }
                true
            }
            _ => false,
        }
    }

    /// Handle Escape key for navigation back or quit.
    /// Returns true if handled.
    fn handle_escape_key(&mut self) -> bool {
        // Skip if any modal is active (let ContentPanel handle Esc)
        if self.is_modal_active() {
            return false;
        }

        // Navigate back based on current view
        if self.content.current_view() == ContentView::QueryResult {
            let _ = self.action_tx.send(AppAction::BackToQuestions);
        } else if self.content.is_collection_questions_view() {
            let _ = self.action_tx.send(AppAction::BackToCollections);
        } else if self.content.is_database_schemas_view() {
            let _ = self.action_tx.send(AppAction::BackToDatabases);
        } else if self.content.is_schema_tables_view() {
            let _ = self.action_tx.send(AppAction::BackToSchemas);
        } else if self.content.is_table_preview_view() {
            let _ = self.action_tx.send(AppAction::BackToTables);
        } else if self.content.get_active_search().is_some() {
            self.content.clear_search();
            let _ = self
                .action_tx
                .send(AppAction::LoadData(DataRequest::Questions));
        } else {
            self.should_quit = true;
        }
        true
    }

    /// Handle refresh action for current view.
    fn handle_refresh(&mut self) {
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
    }

    /// Handle Enter key for content-specific actions (execute, drill-down, detail).
    /// Returns true if the key was handled.
    pub(super) fn handle_enter_key(&mut self) -> bool {
        let view = self.content.current_view();
        let modal_active =
            self.content.is_sort_mode_active() || self.content.is_filter_mode_active();

        match view {
            ContentView::Questions => {
                if let Some(question_id) = self.content.get_selected_question_id() {
                    let _ = self.action_tx.send(AppAction::ExecuteQuestion(question_id));
                    return true;
                }
            }
            ContentView::Collections => {
                if let Some((id, name)) = self.content.get_selected_collection_info() {
                    let _ = self
                        .action_tx
                        .send(AppAction::DrillDownCollection(id, name));
                    return true;
                }
            }
            ContentView::Databases => {
                if let Some((id, name)) = self.content.get_selected_database_info() {
                    let _ = self.action_tx.send(AppAction::DrillDownDatabase(id, name));
                    return true;
                }
            }
            ContentView::QueryResult if !modal_active => {
                if let Some((columns, values)) = self.content.get_selected_record() {
                    self.record_detail = Some(RecordDetailOverlay::new(columns, values));
                    self.show_record_detail = true;
                    return true;
                }
            }
            _ => {}
        }

        // Handle drill-down views
        if self.content.is_collection_questions_view() {
            if let Some(question_id) = self.content.get_selected_question_id() {
                let _ = self.action_tx.send(AppAction::ExecuteQuestion(question_id));
                return true;
            }
        }
        if self.content.is_database_schemas_view() {
            if let Some(schema_name) = self.content.get_selected_schema() {
                let _ = self.action_tx.send(AppAction::DrillDownSchema(schema_name));
                return true;
            }
        }
        if self.content.is_schema_tables_view() {
            if let Some((table_id, table_name)) = self.content.get_selected_table_info() {
                let _ = self
                    .action_tx
                    .send(AppAction::DrillDownTable(table_id, table_name));
                return true;
            }
        }
        if self.content.is_table_preview_view() && !modal_active {
            if let Some((columns, values)) = self.content.get_selected_record() {
                self.record_detail = Some(RecordDetailOverlay::new(columns, values));
                self.show_record_detail = true;
                return true;
            }
        }

        false
    }

    /// Execute copy with the specified format.
    fn execute_copy(&mut self, format: CopyFormat) {
        if let Some(ref menu) = self.copy_menu {
            let (columns, values) = menu.record_data();
            let include_header = menu.include_header();

            let text = match format {
                CopyFormat::Json => format_record_json(columns, values),
                CopyFormat::Csv => format_record_csv(columns, values, include_header),
                CopyFormat::Tsv => format_record_tsv(columns, values, include_header),
            };

            match copy_to_clipboard(&text) {
                Ok(()) => {
                    self.status_bar
                        .set_message(format!("Copied as {}", format.label()));
                }
                Err(e) => {
                    self.status_bar.set_message(format!("Copy failed: {}", e));
                }
            }
        }

        self.close_copy_menu();
    }

    /// Close the copy menu.
    fn close_copy_menu(&mut self) {
        self.show_copy_menu = false;
        self.copy_menu = None;
    }
}
