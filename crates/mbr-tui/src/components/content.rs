//! Content panel component.
//!
//! Displays the main content area (query results, question details, etc.).

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
};

use mbr_core::api::models::Question;

use super::{Component, ScrollState};
use crate::layout::questions_table::{COLLECTION_WIDTH, ID_WIDTH, NAME_MIN_WIDTH};
use crate::service::LoadState;

/// Content view types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContentView {
    #[default]
    Welcome,
    Questions,
    Collections,
    Databases,
    QueryResult,
}

/// Query result data for display in TUI.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryResultData {
    /// Question ID that was executed
    pub question_id: u32,
    /// Question name for display
    pub question_name: String,
    /// Column headers
    pub columns: Vec<String>,
    /// Row data (each cell as string)
    pub rows: Vec<Vec<String>>,
}

/// Default rows per page for query result pagination.
const DEFAULT_ROWS_PER_PAGE: usize = 100;

/// Content panel showing main content.
pub struct ContentPanel {
    view: ContentView,
    scroll: ScrollState,
    /// Horizontal scroll offset (column index)
    scroll_x: usize,
    /// Questions data for the Questions view
    questions: LoadState<Vec<Question>>,
    /// Table state for Questions view (manages selection and scroll)
    table_state: TableState,
    /// Query result data for QueryResult view
    query_result: Option<QueryResultData>,
    /// Table state for query result table
    result_table_state: TableState,
    /// Current page for query result pagination (0-indexed)
    result_page: usize,
    /// Rows per page for query result pagination
    rows_per_page: usize,
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
            query_result: None,
            result_table_state: TableState::default(),
            result_page: 0,
            rows_per_page: DEFAULT_ROWS_PER_PAGE,
        }
    }

    /// Set the current view.
    pub fn set_view(&mut self, view: ContentView) {
        self.view = view;
        self.scroll = ScrollState::default();
        self.scroll_x = 0;
        self.table_state = TableState::default();
    }

    /// Get the current view.
    pub fn current_view(&self) -> ContentView {
        self.view
    }

    /// Update questions data from AppData.
    /// Automatically selects first item when data is loaded.
    pub fn update_questions(&mut self, questions: &LoadState<Vec<Question>>) {
        self.questions = questions.clone();

        // Auto-select first item when data is loaded
        if let LoadState::Loaded(items) = questions {
            if !items.is_empty() && self.table_state.selected().is_none() {
                self.table_state.select(Some(0));
            }
        }
    }

    /// Select next question in list.
    pub fn select_next(&mut self) {
        if let LoadState::Loaded(questions) = &self.questions {
            if questions.is_empty() {
                return;
            }
            let current = self.table_state.selected().unwrap_or(0);
            let next = (current + 1).min(questions.len() - 1);
            self.table_state.select(Some(next));
        }
    }

    /// Select previous question in list.
    pub fn select_previous(&mut self) {
        let current = self.table_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.table_state.select(Some(prev));
    }

    /// Select first question in list.
    pub fn select_first(&mut self) {
        self.table_state.select(Some(0));
    }

    /// Select last question in list.
    pub fn select_last(&mut self) {
        if let LoadState::Loaded(questions) = &self.questions {
            if !questions.is_empty() {
                self.table_state.select(Some(questions.len() - 1));
            }
        }
    }

    /// Get the currently selected question ID.
    pub fn get_selected_question_id(&self) -> Option<u32> {
        if self.view != ContentView::Questions {
            return None;
        }
        if let LoadState::Loaded(questions) = &self.questions {
            if let Some(selected) = self.table_state.selected() {
                return questions.get(selected).map(|q| q.id);
            }
        }
        None
    }

    /// Set query result data and switch to QueryResult view.
    pub fn set_query_result(&mut self, data: QueryResultData) {
        self.query_result = Some(data);
        self.result_table_state = TableState::default();
        self.result_page = 0; // Reset to first page
        self.scroll_x = 0;
        // Auto-select first row if available
        if self
            .query_result
            .as_ref()
            .is_some_and(|r| !r.rows.is_empty())
        {
            self.result_table_state.select(Some(0));
        }
        self.view = ContentView::QueryResult;
    }

    /// Clear query result and return to Questions view.
    pub fn back_to_questions(&mut self) {
        self.query_result = None;
        self.result_table_state = TableState::default();
        self.result_page = 0;
        self.scroll_x = 0;
        self.view = ContentView::Questions;
    }

    /// Get total number of pages for query result.
    fn total_pages(&self) -> usize {
        self.query_result
            .as_ref()
            .map(|r| (r.rows.len() + self.rows_per_page - 1) / self.rows_per_page)
            .unwrap_or(0)
    }

    /// Go to next page in query result.
    fn next_page(&mut self) {
        let total = self.total_pages();
        if total > 0 && self.result_page < total - 1 {
            self.result_page += 1;
            self.result_table_state.select(Some(0)); // Reset selection to first row of new page
        }
    }

    /// Go to previous page in query result.
    fn prev_page(&mut self) {
        if self.result_page > 0 {
            self.result_page -= 1;
            self.result_table_state.select(Some(0)); // Reset selection to first row of new page
        }
    }

    /// Go to first page in query result.
    fn first_page(&mut self) {
        self.result_page = 0;
        self.result_table_state.select(Some(0));
    }

    /// Go to last page in query result.
    fn last_page(&mut self) {
        let total = self.total_pages();
        if total > 0 {
            self.result_page = total - 1;
            self.result_table_state.select(Some(0));
        }
    }

    /// Scroll left (show previous columns).
    fn scroll_left(&mut self) {
        self.scroll_x = self.scroll_x.saturating_sub(1);
    }

    /// Scroll right (show next columns).
    fn scroll_right(&mut self) {
        let total_cols = self.get_total_columns();
        if total_cols > 0 && self.scroll_x < total_cols.saturating_sub(1) {
            self.scroll_x += 1;
        }
    }

    /// Get total number of columns for current view.
    fn get_total_columns(&self) -> usize {
        match self.view {
            ContentView::QueryResult => self
                .query_result
                .as_ref()
                .map(|r| r.columns.len())
                .unwrap_or(0),
            ContentView::Questions => 3, // ID, Name, Collection
            _ => 0,
        }
    }

    /// Navigate result table.
    fn select_result_next(&mut self) {
        if let Some(ref result) = self.query_result {
            if result.rows.is_empty() {
                return;
            }
            let current = self.result_table_state.selected().unwrap_or(0);
            let next = (current + 1).min(result.rows.len() - 1);
            self.result_table_state.select(Some(next));
        }
    }

    fn select_result_previous(&mut self) {
        let current = self.result_table_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.result_table_state.select(Some(prev));
    }

    /// Get the number of rows in the current page.
    fn current_page_row_count(&self) -> usize {
        self.query_result
            .as_ref()
            .map(|r| {
                let total_rows = r.rows.len();
                let page_start = self.result_page * self.rows_per_page;
                let page_end = (page_start + self.rows_per_page).min(total_rows);
                page_end - page_start
            })
            .unwrap_or(0)
    }

    /// Scroll result table up by multiple rows (PageUp).
    fn scroll_result_page_up(&mut self) {
        const SCROLL_AMOUNT: usize = 10;
        let current = self.result_table_state.selected().unwrap_or(0);
        let new = current.saturating_sub(SCROLL_AMOUNT);
        self.result_table_state.select(Some(new));
    }

    /// Scroll result table down by multiple rows (PageDown).
    fn scroll_result_page_down(&mut self) {
        const SCROLL_AMOUNT: usize = 10;
        let current = self.result_table_state.selected().unwrap_or(0);
        let page_row_count = self.current_page_row_count();
        let new = (current + SCROLL_AMOUNT).min(page_row_count.saturating_sub(1));
        self.result_table_state.select(Some(new));
    }

    /// Render welcome view content.
    fn render_welcome(&self, _area: Rect, focused: bool) -> Paragraph<'static> {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let banner = r#"
                 _                  _           _
  _ __ ___  | |__  _ __       | |_ _   _ (_)
 | '_ ` _ \ | '_ \| '__|_____ | __| | | || |
 | | | | | || |_) | |  |_____|| |_| |_| || |
 |_| |_| |_||_.__/|_|         \__|\__,_||_|
"#;

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(""));

        for banner_line in banner.lines() {
            lines.push(Line::from(Span::styled(
                banner_line.to_string(),
                Style::default().fg(Color::Cyan),
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Welcome to mbr-tui!",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Select an item from the navigation panel to get started.",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from("  Quick Keys:"));
        lines.push(Line::from(Span::styled(
            "    Tab       - Switch panels",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(Span::styled(
            "    ↑/↓ j/k   - Navigate items",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(Span::styled(
            "    Enter     - Select item",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(Span::styled(
            "    q         - Quit",
            Style::default().fg(Color::Yellow),
        )));

        Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Welcome ")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .wrap(Wrap { trim: false })
    }

    /// Render questions view with table.
    fn render_questions(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        match &self.questions {
            LoadState::Idle => {
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Press 'r' to load questions",
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .block(
                    Block::default()
                        .title(" Questions ")
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );
                frame.render_widget(paragraph, area);
            }
            LoadState::Loading => {
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  ⏳ Loading questions...",
                        Style::default().fg(Color::Yellow),
                    )),
                ])
                .block(
                    Block::default()
                        .title(" Questions ")
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );
                frame.render_widget(paragraph, area);
            }
            LoadState::Error(msg) => {
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("  ❌ Error: {}", msg),
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
                        .title(" Questions ")
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );
                frame.render_widget(paragraph, area);
            }
            LoadState::Loaded(questions) => {
                if questions.is_empty() {
                    let paragraph = Paragraph::new(vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            "  No questions found",
                            Style::default().fg(Color::DarkGray),
                        )),
                    ])
                    .block(
                        Block::default()
                            .title(" Questions (0) ")
                            .borders(Borders::ALL)
                            .border_style(border_style),
                    );
                    frame.render_widget(paragraph, area);
                } else {
                    // Create table rows (no manual styling - TableState handles highlight)
                    let rows: Vec<Row> = questions
                        .iter()
                        .map(|q| {
                            let collection_name = q
                                .collection
                                .as_ref()
                                .map(|c| c.name.as_str())
                                .unwrap_or("—");

                            Row::new(vec![
                                Cell::from(format!("{}", q.id)),
                                Cell::from(q.name.clone()),
                                Cell::from(collection_name.to_string()),
                            ])
                        })
                        .collect();

                    let table = Table::new(
                        rows,
                        [
                            Constraint::Length(ID_WIDTH),
                            Constraint::Min(NAME_MIN_WIDTH),
                            Constraint::Length(COLLECTION_WIDTH),
                        ],
                    )
                    .header(
                        Row::new(vec!["ID", "Name", "Collection"])
                            .style(
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            )
                            .bottom_margin(1),
                    )
                    .block(
                        Block::default()
                            .title(format!(" Questions ({}) ", questions.len()))
                            .borders(Borders::ALL)
                            .border_style(border_style),
                    )
                    .row_highlight_style(
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("► ");

                    // Use stateful widget for automatic scroll management
                    frame.render_stateful_widget(table, area, &mut self.table_state);
                }
            }
        }
    }

    /// Render placeholder for unimplemented views.
    fn render_placeholder(&self, title: &str, focused: bool) -> Paragraph<'static> {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {} view", title),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  ⚠ Not implemented yet",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  This feature is planned for future releases.",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        Paragraph::new(lines)
            .block(
                Block::default()
                    .title(format!(" {} ", title))
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .wrap(Wrap { trim: false })
    }

    /// Render query result view with table.
    fn render_query_result(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        match &self.query_result {
            None => {
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  No query result available",
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .block(
                    Block::default()
                        .title(" Query Result ")
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );
                frame.render_widget(paragraph, area);
            }
            Some(result) => {
                if result.rows.is_empty() {
                    let paragraph = Paragraph::new(vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            format!("  Query: {}", result.question_name),
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            "  No data returned",
                            Style::default().fg(Color::DarkGray),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            "  Press Esc to go back",
                            Style::default().fg(Color::Yellow),
                        )),
                    ])
                    .block(
                        Block::default()
                            .title(format!(" Query Result: {} (0 rows) ", result.question_name))
                            .borders(Borders::ALL)
                            .border_style(border_style),
                    );
                    frame.render_widget(paragraph, area);
                } else {
                    // Pagination: calculate row range for current page
                    let total_rows = result.rows.len();
                    let total_pages = self.total_pages();
                    let page_start = self.result_page * self.rows_per_page;
                    let page_end = (page_start + self.rows_per_page).min(total_rows);
                    let page_rows = &result.rows[page_start..page_end];

                    // Calculate visible columns based on scroll_x
                    let total_cols = result.columns.len();
                    let scroll_x = self.scroll_x.min(total_cols.saturating_sub(1));

                    // Calculate how many columns can fit (estimate based on min width)
                    let available_width = area.width.saturating_sub(4) as usize; // borders + highlight symbol
                    let min_col_width = 15usize;
                    let visible_cols = (available_width / min_col_width).max(1).min(total_cols);
                    let end_col = (scroll_x + visible_cols).min(total_cols);

                    // Slice columns based on scroll position
                    let visible_columns: Vec<String> = result.columns[scroll_x..end_col].to_vec();
                    let visible_col_count = visible_columns.len();

                    // Create dynamic column widths
                    let constraints: Vec<Constraint> = if visible_col_count <= 3 {
                        visible_columns
                            .iter()
                            .map(|_| Constraint::Ratio(1, visible_col_count as u32))
                            .collect()
                    } else {
                        visible_columns
                            .iter()
                            .map(|_| Constraint::Min(15))
                            .collect()
                    };

                    // Create table rows with sliced cells (only current page)
                    let rows: Vec<Row> = page_rows
                        .iter()
                        .map(|row| {
                            let cells: Vec<Cell> = row[scroll_x..end_col.min(row.len())]
                                .iter()
                                .map(|cell| Cell::from(cell.clone()))
                                .collect();
                            Row::new(cells)
                        })
                        .collect();

                    // Create header row
                    let header_cells: Vec<Cell> = visible_columns
                        .iter()
                        .map(|col| Cell::from(col.clone()))
                        .collect();

                    // Build column indicator (e.g., "← 1-5 of 12 →")
                    let col_indicator = if total_cols > visible_cols {
                        let left_arrow = if scroll_x > 0 { "← " } else { "  " };
                        let right_arrow = if end_col < total_cols { " →" } else { "  " };
                        format!(
                            " {}Col {}-{}/{}{}",
                            left_arrow,
                            scroll_x + 1,
                            end_col,
                            total_cols,
                            right_arrow
                        )
                    } else {
                        String::new()
                    };

                    // Build page indicator
                    let page_indicator = if total_pages > 1 {
                        format!(
                            " Page {}/{} (rows {}-{} of {})",
                            self.result_page + 1,
                            total_pages,
                            page_start + 1,
                            page_end,
                            total_rows
                        )
                    } else {
                        format!(" {} rows", total_rows)
                    };

                    let table = Table::new(rows, constraints)
                        .header(
                            Row::new(header_cells)
                                .style(
                                    Style::default()
                                        .fg(Color::Yellow)
                                        .add_modifier(Modifier::BOLD),
                                )
                                .bottom_margin(1),
                        )
                        .block(
                            Block::default()
                                .title(format!(
                                    " {}{}{}",
                                    result.question_name,
                                    page_indicator,
                                    col_indicator
                                ))
                                .borders(Borders::ALL)
                                .border_style(border_style),
                        )
                        .row_highlight_style(
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol("► ");

                    frame.render_stateful_widget(table, area, &mut self.result_table_state);
                }
            }
        }
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
            ContentView::QueryResult => {
                self.render_query_result(area, frame, focused);
                return;
            }
            _ => {}
        }

        // Other views return Paragraph widgets
        let widget = match self.view {
            ContentView::Welcome => self.render_welcome(area, focused),
            ContentView::Questions | ContentView::QueryResult => unreachable!(), // Handled above
            ContentView::Collections => self.render_placeholder("Collections", focused),
            ContentView::Databases => self.render_placeholder("Databases", focused),
        };

        frame.render_widget(widget, area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Questions view has list navigation
        if self.view == ContentView::Questions {
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
        } else if self.view == ContentView::QueryResult {
            // QueryResult view has result table navigation + horizontal scroll + pagination
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
                // Note: Esc is handled in App for returning to Questions
                _ => false,
            }
        } else {
            // Other views use scroll
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
    }

    fn title(&self) -> &str {
        match self.view {
            ContentView::Welcome => "Welcome",
            ContentView::Questions => "Questions",
            ContentView::Collections => "Collections",
            ContentView::Databases => "Databases",
            ContentView::QueryResult => "Query Result",
        }
    }
}
