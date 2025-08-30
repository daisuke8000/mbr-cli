use crate::api::models::{
    Collection, CollectionDetail, CollectionStats, Dashboard, DashboardCard, QueryResult, Question,
};
use crate::error::AppError;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, Write};

/// Interactive display manager for full-screen table and pagination
#[derive(Default)]
pub struct InteractiveDisplay;

impl InteractiveDisplay {
    pub fn new() -> Self {
        Self
    }

    /// Display query results with interactive pagination
    /// Based on original implementation: RAW mode + Alternate Screen + interactive display with scroll functionality
    pub async fn display_query_result_pagination(
        &self,
        result: &QueryResult,
        page_size: usize,
        initial_offset: Option<usize>,
        no_fullscreen: bool,
        question_id: u32,
        question_name: &str,
    ) -> Result<(), AppError> {
        use crossterm::terminal::{
            EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size,
        };
        use std::io::{self, Write};

        // RAII cleanup structures
        struct RawModeCleanup;
        impl Drop for RawModeCleanup {
            fn drop(&mut self) {
                let _ = disable_raw_mode();
            }
        }

        struct ScreenCleanup;
        impl Drop for ScreenCleanup {
            fn drop(&mut self) {
                let _ = execute!(io::stdout(), LeaveAlternateScreen);
            }
        }

        if no_fullscreen {
            // Simple mode fallback
            let display = crate::display::table::TableDisplay::new();
            let table_output = display.render_query_result(result)?;
            println!("{}", table_output);
            return Ok(());
        }

        // Full screen mode - RAW mode + Alternate Screen + scroll
        match enable_raw_mode() {
            Ok(()) => {
                let _cleanup = RawModeCleanup;
                execute!(io::stdout(), EnterAlternateScreen).ok();
                let _screen_cleanup = ScreenCleanup;

                // Get terminal size
                let (_terminal_width, terminal_height) = size().unwrap_or((80, 24));

                // Pagination state
                let total_rows = result.data.rows.len();
                let base_offset = 0; // Use 0 here since the result is already offset-adjusted
                let available_rows = total_rows.saturating_sub(base_offset);
                let total_pages = if available_rows == 0 {
                    1
                } else {
                    available_rows.div_ceil(page_size)
                }; // Total pages considering offset
                let mut current_page = 1; // Initial display always starts from page 1

                // Table renderer for display
                let display = crate::display::table::TableDisplay::new();

                // Scroll state (for scrolling within table)
                let mut scroll_offset = 0;
                // Reserve 8 lines: header space (5 lines) + prompt space (3 lines)
                let available_height = terminal_height.saturating_sub(8) as usize;

                loop {
                    // Get current page data (considering offset)
                    let start_row = base_offset + (current_page - 1) * page_size;
                    let end_row = (start_row + page_size).min(total_rows);

                    // Create QueryResult limited to current page data
                    let page_rows = if start_row < total_rows {
                        result.data.rows[start_row..end_row].to_vec()
                    } else {
                        vec![]
                    };

                    let page_result = crate::api::models::QueryResult {
                        data: crate::api::models::QueryData {
                            cols: result.data.cols.clone(),
                            rows: page_rows,
                        },
                    };

                    // Generate table for current page
                    let page_table_output = display.render_query_result(&page_result)?;
                    let table_lines: Vec<&str> = page_table_output.lines().collect();

                    // Clear entire screen
                    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();

                    // Display header (fixed)
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Cyan),
                        Print(format!("Question {}: {}", question_id, question_name)),
                        ResetColor,
                        Print("\r\n"),
                        SetForegroundColor(Color::Yellow),
                        Print(format!(
                            "Page {}/{} | Showing rows {}-{} of {} total (available: {}, offset: {}) | Page size: {}",
                            current_page,
                            total_pages,
                            initial_offset.unwrap_or(0) + (current_page - 1) * page_size + 1,  // Correct user display range start
                            initial_offset.unwrap_or(0) + (current_page - 1) * page_size + (end_row - start_row),  // Correct user display range end
                            total_rows,
                            available_rows,
                            initial_offset.unwrap_or(0),
                            page_size
                        )),
                        ResetColor,
                        Print("\r\n\r\n")
                    ).ok();

                    // Display content within scroll range
                    let total_lines = table_lines.len();
                    let start_line = scroll_offset.min(total_lines);
                    let end_line = (start_line + available_height).min(total_lines);

                    if start_line < total_lines {
                        let display_lines = &table_lines[start_line..end_line];
                        for line in display_lines {
                            println!("{}\r", line);
                        }
                    }

                    // Clear bottom of screen (prevent leftover characters)
                    execute!(io::stdout(), Clear(ClearType::FromCursorDown)).ok();

                    // Display prompt (fixed at bottom)
                    execute!(
                        io::stdout(),
                        cursor::MoveTo(0, terminal_height - 2),
                        SetForegroundColor(Color::Green),
                        Print("Controls: ↑↓/jk=scroll | n/p=page | Home/End | q=quit | h=help"),
                        ResetColor
                    )
                    .ok();

                    io::stdout().flush().ok();

                    // Key input processing
                    if let Ok(Event::Key(KeyEvent {
                        code, modifiers, ..
                    })) = event::read()
                    {
                        match code {
                            // Exit
                            KeyCode::Char('q') | KeyCode::Char('Q') => break,
                            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                                break;
                            }
                            KeyCode::Esc => break,

                            // Scroll (line by line)
                            KeyCode::Up | KeyCode::Char('k') => {
                                scroll_offset = scroll_offset.saturating_sub(1);
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                let max_offset = total_lines.saturating_sub(available_height);
                                scroll_offset = (scroll_offset + 1).min(max_offset);
                            }

                            // Page navigation (data pages)
                            KeyCode::Char('n') => {
                                if current_page < total_pages {
                                    current_page += 1;
                                    scroll_offset = 0; // Reset scroll position for new page
                                }
                            }
                            KeyCode::Char('p') => {
                                if current_page > 1 {
                                    current_page -= 1;
                                    scroll_offset = 0; // Reset scroll position for new page
                                }
                            }

                            // Scroll movement (within page)
                            KeyCode::PageUp => {
                                scroll_offset = scroll_offset.saturating_sub(available_height);
                            }
                            KeyCode::PageDown => {
                                let max_offset = total_lines.saturating_sub(available_height);
                                scroll_offset = (scroll_offset + available_height).min(max_offset);
                            }

                            // First/last (page navigation)
                            KeyCode::Home => {
                                current_page = 1;
                                scroll_offset = 0;
                            }
                            KeyCode::End => {
                                current_page = total_pages.max(1);
                                scroll_offset = 0;
                            }

                            // Show help
                            KeyCode::Char('h') | KeyCode::Char('H') => {
                                execute!(
                                    io::stdout(),
                                    Clear(ClearType::All),
                                    cursor::MoveTo(0, 0),
                                    SetForegroundColor(Color::Cyan),
                                    Print("Keyboard Navigation Help"),
                                    ResetColor,
                                    Print("\r\n\r\n"),
                                    Print("Page Navigation:\r\n"),
                                    Print("  n           : Next page\r\n"),
                                    Print("  p           : Previous page\r\n"),
                                    Print("  Home        : First page\r\n"),
                                    Print("  End         : Last page\r\n"),
                                    Print("\r\n"),
                                    Print("Scroll Controls (within page):\r\n"),
                                    Print("  ↑, k        : Scroll up (1 line)\r\n"),
                                    Print("  ↓, j        : Scroll down (1 line)\r\n"),
                                    Print("  Page Up     : Scroll up (page)\r\n"),
                                    Print("  Page Down   : Scroll down (page)\r\n"),
                                    Print("\r\n"),
                                    Print("Other Controls:\r\n"),
                                    Print("  q, Q, Esc  : Quit\r\n"),
                                    Print("  Ctrl+C     : Force quit\r\n"),
                                    Print("  h, H       : Show this help\r\n"),
                                    Print("\r\n"),
                                    SetForegroundColor(Color::Yellow),
                                    Print("Press any key to continue..."),
                                    ResetColor
                                )
                                .ok();
                                io::stdout().flush().ok();
                                event::read().ok();
                            }

                            _ => {} // Ignore invalid keys
                        }
                    }
                }
            }
            Err(_) => {
                // Fallback when RAW mode fails
                println!("Warning: Could not enable full-screen mode, falling back to simple mode");
                let display = crate::display::table::TableDisplay::new();
                let table_output = display.render_query_result(result)?;
                println!("{}", table_output);
            }
        }

        Ok(())
    }

    /// Display question list with interactive pagination
    /// RAW mode + Alternate Screen + pagination display for Question List
    pub async fn display_question_list_pagination(
        &self,
        questions: &[Question],
        page_size: usize,
    ) -> Result<(), AppError> {
        use crossterm::terminal::{
            EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size,
        };
        use std::io::{self, Write};

        // RAII cleanup structures
        struct RawModeCleanup;
        impl Drop for RawModeCleanup {
            fn drop(&mut self) {
                let _ = disable_raw_mode();
            }
        }

        struct ScreenCleanup;
        impl Drop for ScreenCleanup {
            fn drop(&mut self) {
                let _ = execute!(io::stdout(), LeaveAlternateScreen);
            }
        }

        // Full screen mode - RAW mode + Alternate Screen + pagination (always used)
        match enable_raw_mode() {
            Ok(()) => {
                let _cleanup = RawModeCleanup;
                execute!(io::stdout(), EnterAlternateScreen).ok();
                let _screen_cleanup = ScreenCleanup;

                // Get terminal size
                let (_terminal_width, terminal_height) = size().unwrap_or((80, 24));

                // Pagination state
                let total_questions = questions.len();
                let total_pages = if total_questions == 0 {
                    1
                } else {
                    total_questions.div_ceil(page_size)
                };
                let mut current_page = 1;

                // Table renderer for display
                let display = crate::display::table::TableDisplay::new();

                // Scroll state (for scrolling within table)
                let mut scroll_offset = 0;
                // Reserve 6 lines: header space (3 lines) + prompt space (3 lines)
                let available_height = terminal_height.saturating_sub(6) as usize;

                loop {
                    // Get current page data
                    let start_idx = (current_page - 1) * page_size;
                    let end_idx = (start_idx + page_size).min(total_questions);

                    let page_questions = if start_idx < total_questions {
                        &questions[start_idx..end_idx]
                    } else {
                        &[]
                    };

                    // Generate table for current page
                    let page_table_output = display.render_question_list(page_questions)?;
                    let table_lines: Vec<&str> = page_table_output.lines().collect();

                    // Clear entire screen
                    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();

                    // Display header (fixed)
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Cyan),
                        Print("Question List"),
                        ResetColor,
                        Print("\r\n"),
                        SetForegroundColor(Color::Yellow),
                        Print(format!(
                            "Page {}/{} | Showing questions {}-{} of {} total | Page size: {}",
                            current_page,
                            total_pages,
                            start_idx + 1,
                            start_idx + page_questions.len(),
                            total_questions,
                            page_size
                        )),
                        ResetColor,
                        Print("\r\n\r\n")
                    )
                    .ok();

                    // Display content within scroll range
                    let total_lines = table_lines.len();
                    let start_line = scroll_offset.min(total_lines);
                    let end_line = (start_line + available_height).min(total_lines);

                    if start_line < total_lines {
                        let display_lines = &table_lines[start_line..end_line];
                        for line in display_lines {
                            println!("{}\r", line);
                        }
                    }

                    // Clear bottom of screen (prevent leftover characters)
                    execute!(io::stdout(), Clear(ClearType::FromCursorDown)).ok();

                    // Display prompt (fixed at bottom)
                    execute!(
                        io::stdout(),
                        cursor::MoveTo(0, terminal_height - 2),
                        SetForegroundColor(Color::Green),
                        Print("Controls: ↑↓/jk=scroll | n/p=page | Home/End | q=quit | h=help"),
                        ResetColor
                    )
                    .ok();

                    io::stdout().flush().ok();

                    // Key input processing
                    if let Ok(Event::Key(KeyEvent {
                        code, modifiers, ..
                    })) = event::read()
                    {
                        match code {
                            // Exit
                            KeyCode::Char('q') | KeyCode::Char('Q') => break,
                            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                                break;
                            }
                            KeyCode::Esc => break,

                            // Scroll (line by line)
                            KeyCode::Up | KeyCode::Char('k') => {
                                scroll_offset = scroll_offset.saturating_sub(1);
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                let max_offset = total_lines.saturating_sub(available_height);
                                scroll_offset = (scroll_offset + 1).min(max_offset);
                            }

                            // Page navigation
                            KeyCode::Char('n') => {
                                if current_page < total_pages {
                                    current_page += 1;
                                    scroll_offset = 0; // Reset scroll position for new page
                                }
                            }
                            KeyCode::Char('p') => {
                                if current_page > 1 {
                                    current_page -= 1;
                                    scroll_offset = 0; // Reset scroll position for new page
                                }
                            }

                            // Scroll movement (within page)
                            KeyCode::PageUp => {
                                scroll_offset = scroll_offset.saturating_sub(available_height);
                            }
                            KeyCode::PageDown => {
                                let max_offset = total_lines.saturating_sub(available_height);
                                scroll_offset = (scroll_offset + available_height).min(max_offset);
                            }

                            // First/last
                            KeyCode::Home => {
                                current_page = 1;
                                scroll_offset = 0;
                            }
                            KeyCode::End => {
                                current_page = total_pages.max(1);
                                scroll_offset = 0;
                            }

                            // Show help
                            KeyCode::Char('h') | KeyCode::Char('H') => {
                                execute!(
                                    io::stdout(),
                                    Clear(ClearType::All),
                                    cursor::MoveTo(0, 0),
                                    SetForegroundColor(Color::Cyan),
                                    Print("Question List - Keyboard Navigation Help"),
                                    ResetColor,
                                    Print("\r\n\r\n"),
                                    Print("Page Navigation:\r\n"),
                                    Print("  n           : Next page\r\n"),
                                    Print("  p           : Previous page\r\n"),
                                    Print("  Home        : First page\r\n"),
                                    Print("  End         : Last page\r\n"),
                                    Print("\r\n"),
                                    Print("Scroll Controls (within page):\r\n"),
                                    Print("  ↑, k        : Scroll up (1 line)\r\n"),
                                    Print("  ↓, j        : Scroll down (1 line)\r\n"),
                                    Print("  Page Up     : Scroll up (page)\r\n"),
                                    Print("  Page Down   : Scroll down (page)\r\n"),
                                    Print("\r\n"),
                                    Print("Other Controls:\r\n"),
                                    Print("  q, Q, Esc  : Quit\r\n"),
                                    Print("  Ctrl+C     : Force quit\r\n"),
                                    Print("  h, H       : Show this help\r\n"),
                                    Print("\r\n"),
                                    SetForegroundColor(Color::Yellow),
                                    Print("Press any key to continue..."),
                                    ResetColor
                                )
                                .ok();
                                io::stdout().flush().ok();
                                event::read().ok();
                            }

                            _ => {} // Ignore invalid keys
                        }
                    }
                }
            }
            Err(_) => {
                // Fallback when RAW mode fails
                println!("Warning: Could not enable full-screen mode, falling back to simple mode");
                let display = crate::display::table::TableDisplay::new();
                let table_output = display.render_question_list(questions)?;
                println!("{}", table_output);
            }
        }

        Ok(())
    }

    /// Display dashboard list with interactive pagination
    /// RAW mode + Alternate Screen + pagination display for Dashboard List
    pub async fn display_dashboard_list_pagination(
        &self,
        dashboards: &[Dashboard],
        page_size: usize,
    ) -> Result<(), AppError> {
        use crossterm::terminal::{
            EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size,
        };
        use std::io::{self, Write};

        // RAII cleanup structures
        struct RawModeCleanup;
        impl Drop for RawModeCleanup {
            fn drop(&mut self) {
                let _ = disable_raw_mode();
            }
        }

        struct ScreenCleanup;
        impl Drop for ScreenCleanup {
            fn drop(&mut self) {
                let _ = execute!(io::stdout(), LeaveAlternateScreen);
            }
        }

        // Full screen mode - RAW mode + Alternate Screen + scroll
        match enable_raw_mode() {
            Ok(()) => {
                let _cleanup = RawModeCleanup;
                execute!(io::stdout(), EnterAlternateScreen).ok();
                let _screen_cleanup = ScreenCleanup;

                // Get terminal size
                let (_terminal_width, terminal_height) = size().unwrap_or((80, 24));

                // Pagination state
                let total_dashboards = dashboards.len();
                let total_pages = if total_dashboards == 0 {
                    1
                } else {
                    total_dashboards.div_ceil(page_size)
                };
                let mut current_page = 1;

                // Scroll state (for scrolling within table)
                let mut scroll_offset = 0;
                // Reserve 8 lines: header space (5 lines) + prompt space (3 lines)
                let available_height = terminal_height.saturating_sub(8) as usize;

                loop {
                    // Get current page data
                    let start_idx = (current_page - 1) * page_size;
                    let end_idx = (start_idx + page_size).min(total_dashboards);
                    let page_dashboards = if start_idx < total_dashboards {
                        &dashboards[start_idx..end_idx]
                    } else {
                        &[]
                    };

                    // Generate dashboard table lines with text wrapping
                    let mut table_lines = vec![
                        "┌──────┬─────────────────────────────────┬─────────────────────────────────┬──────────────────┬──────────────────┐".to_string(),
                        "│ ID   │ Name                            │ Description                     │ Collection       │ Updated          │".to_string(),
                        "├──────┼─────────────────────────────────┼─────────────────────────────────┼──────────────────┼──────────────────┤".to_string(),
                    ];

                    for dashboard in page_dashboards {
                        let name_wrapped = self.wrap_text(&dashboard.name, 31);
                        let desc_wrapped = dashboard
                            .description
                            .as_ref()
                            .map(|d| self.wrap_text(d, 31))
                            .unwrap_or_else(|| vec!["".to_string()]);
                        let collection_text = dashboard
                            .collection_id
                            .map(|id| format!("ID: {}", id))
                            .unwrap_or_else(|| "Personal".to_string());
                        let collection_wrapped = self.wrap_text(&collection_text, 16);
                        let updated_wrapped =
                            self.wrap_text(&self.format_datetime(&dashboard.updated_at), 16);

                        // Find maximum lines needed for this row
                        let max_lines = name_wrapped
                            .len()
                            .max(desc_wrapped.len())
                            .max(collection_wrapped.len())
                            .max(updated_wrapped.len());

                        // Generate multi-line row
                        for line_idx in 0..max_lines {
                            let empty_string = String::new();
                            let name_line = name_wrapped.get(line_idx).unwrap_or(&empty_string);
                            let desc_line = desc_wrapped.get(line_idx).unwrap_or(&empty_string);
                            let collection_line =
                                collection_wrapped.get(line_idx).unwrap_or(&empty_string);
                            let updated_line =
                                updated_wrapped.get(line_idx).unwrap_or(&empty_string);

                            let id_display = if line_idx == 0 {
                                dashboard.id.to_string()
                            } else {
                                String::new()
                            };

                            table_lines.push(format!(
                                "│ {:>4} │ {:31} │ {:31} │ {:16} │ {:16} │",
                                id_display,
                                self.pad_string(name_line, 31),
                                self.pad_string(desc_line, 31),
                                self.pad_string(collection_line, 16),
                                self.pad_string(updated_line, 16)
                            ));
                        }
                    }

                    table_lines.push("└──────┴─────────────────────────────────┴─────────────────────────────────┴──────────────────┴──────────────────┘".to_string());

                    // Clear entire screen
                    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();

                    // Display header (fixed)
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Cyan),
                        Print("Dashboard List"),
                        ResetColor,
                        Print("\r\n"),
                        SetForegroundColor(Color::Yellow),
                        Print(format!(
                            "Page {}/{} | Showing dashboards {}-{} of {} total | Page size: {}",
                            current_page,
                            total_pages,
                            start_idx + 1,
                            end_idx,
                            total_dashboards,
                            page_size
                        )),
                        ResetColor,
                        Print("\r\n\r\n")
                    )
                    .ok();

                    // Display content within scroll range
                    let total_lines = table_lines.len();
                    let start_line = scroll_offset.min(total_lines);
                    let end_line = (start_line + available_height).min(total_lines);

                    if start_line < total_lines {
                        let display_lines = &table_lines[start_line..end_line];
                        for line in display_lines {
                            println!("{}\r", line);
                        }
                    }

                    // Clear bottom of screen
                    execute!(io::stdout(), Clear(ClearType::FromCursorDown)).ok();

                    // Display prompt (fixed at bottom)
                    execute!(
                        io::stdout(),
                        cursor::MoveTo(0, terminal_height - 2),
                        SetForegroundColor(Color::Green),
                        Print("Controls: ↑↓/jk=scroll | n/p=page | Home/End | q=quit | h=help"),
                        ResetColor
                    )
                    .ok();

                    io::stdout().flush().ok();

                    // Key input processing
                    if let Ok(Event::Key(KeyEvent {
                        code, modifiers, ..
                    })) = event::read()
                    {
                        match code {
                            // Exit
                            KeyCode::Char('q') | KeyCode::Char('Q') => break,
                            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                                break;
                            }
                            KeyCode::Esc => break,

                            // Scroll (line by line)
                            KeyCode::Up | KeyCode::Char('k') => {
                                scroll_offset = scroll_offset.saturating_sub(1);
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                let max_offset = total_lines.saturating_sub(available_height);
                                scroll_offset = (scroll_offset + 1).min(max_offset);
                            }

                            // Page navigation
                            KeyCode::Char('n') => {
                                if current_page < total_pages {
                                    current_page += 1;
                                    scroll_offset = 0;
                                }
                            }
                            KeyCode::Char('p') => {
                                if current_page > 1 {
                                    current_page -= 1;
                                    scroll_offset = 0;
                                }
                            }

                            // Scroll movement
                            KeyCode::PageUp => {
                                scroll_offset = scroll_offset.saturating_sub(available_height / 2);
                            }
                            KeyCode::PageDown => {
                                let max_offset = total_lines.saturating_sub(available_height);
                                scroll_offset =
                                    (scroll_offset + available_height / 2).min(max_offset);
                            }
                            KeyCode::Home => {
                                scroll_offset = 0;
                            }
                            KeyCode::End => {
                                scroll_offset = total_lines.saturating_sub(available_height);
                            }

                            // Help
                            KeyCode::Char('h') => {
                                self.show_dashboard_help(terminal_height).await?;
                            }

                            _ => {} // Ignore other keys
                        }
                    }
                }
            }
            Err(_) => {
                // Fallback when RAW mode fails
                println!("Warning: Could not enable full-screen mode, falling back to simple mode");
                println!("Dashboard List ({} found):", dashboards.len());
                for dashboard in dashboards {
                    println!(
                        "  ID: {}, Name: {}, Description: {:?}",
                        dashboard.id, dashboard.name, dashboard.description
                    );
                }
            }
        }

        Ok(())
    }

    // Helper methods for dashboard display
    fn truncate_string(&self, s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            format!("{:width$}", s, width = max_len)
        } else {
            format!("{}…", &s[..max_len.saturating_sub(1)])
        }
    }

    fn format_datetime(&self, datetime: &str) -> String {
        // Simple date formatting - extract date part from ISO datetime
        if let Some(date_part) = datetime.split('T').next() {
            date_part.to_string()
        } else {
            datetime.chars().take(16).collect()
        }
    }

    /// Wrap text to fit within specified width, breaking at word boundaries
    fn wrap_text(&self, text: &str, max_width: usize) -> Vec<String> {
        if text.is_empty() {
            return vec![String::new()];
        }

        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in text.split_whitespace() {
            // If adding this word would exceed the width
            if current_line.len() + word.len() + 1 > max_width {
                if !current_line.is_empty() {
                    lines.push(current_line);
                    current_line = String::new();
                }

                // If a single word is longer than max_width, break it
                if word.len() > max_width {
                    let mut remaining = word;
                    while remaining.len() > max_width {
                        lines.push(remaining[..max_width].to_string());
                        remaining = &remaining[max_width..];
                    }
                    if !remaining.is_empty() {
                        current_line = remaining.to_string();
                    }
                } else {
                    current_line = word.to_string();
                }
            } else {
                if !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(word);
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        }
    }

    /// Pad string to exact width with spaces
    fn pad_string(&self, text: &str, width: usize) -> String {
        format!("{:width$}", text, width = width)
    }

    async fn show_dashboard_help(&self, _terminal_height: u16) -> Result<(), AppError> {
        use std::io::{self, Write};

        execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();

        execute!(
            io::stdout(),
            SetForegroundColor(Color::Cyan),
            Print("Dashboard List - Help\r\n\r\n"),
            ResetColor,
            Print("Navigation:\r\n"),
            Print("  ↑/k      - Scroll up one line\r\n"),
            Print("  ↓/j      - Scroll down one line\r\n"),
            Print("  Page Up  - Scroll up half page\r\n"),
            Print("  Page Down- Scroll down half page\r\n"),
            Print("  Home     - Go to top\r\n"),
            Print("  End      - Go to bottom\r\n"),
            Print("  n        - Next page\r\n"),
            Print("  p        - Previous page\r\n\r\n"),
            Print("Commands:\r\n"),
            Print("  q        - Quit\r\n"),
            Print("  h        - Show this help\r\n\r\n"),
            SetForegroundColor(Color::Green),
            Print("Press any key to return to dashboard list..."),
            ResetColor
        )
        .ok();

        io::stdout().flush().ok();

        // Wait for any key press
        if let Ok(Event::Key(_)) = crossterm::event::read() {
            // Return to main display
        }

        Ok(())
    }

    /// Display dashboard details with interactive features
    pub async fn display_dashboard_details_interactive(
        &self,
        dashboard: &Dashboard,
    ) -> Result<(), AppError> {
        let table_display = crate::display::table::TableDisplay::new();
        let table_content = table_display.render_dashboard_details(dashboard)?;
        let table_lines: Vec<String> = table_content.lines().map(|s| s.to_string()).collect();

        // Enable raw mode for keyboard handling
        match terminal::enable_raw_mode() {
            Ok(_) => {
                let mut scroll_offset = 0;
                let (_, terminal_height) = terminal::size().unwrap_or((80, 24));
                let header_height = 4; // Title + info + blank line
                let footer_height = 2; // Controls + status
                let available_height =
                    (terminal_height as usize).saturating_sub(header_height + footer_height);

                loop {
                    // Clear screen and reset cursor
                    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();

                    // Display header
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Cyan),
                        Print(format!("Dashboard Details - {}", dashboard.name)),
                        ResetColor,
                        Print("\r\n"),
                        SetForegroundColor(Color::Yellow),
                        Print(format!(
                            "ID: {} | Created: {}",
                            dashboard.id, dashboard.created_at
                        )),
                        ResetColor,
                        Print("\r\n\r\n")
                    )
                    .ok();

                    // Display content within scroll range
                    let total_lines = table_lines.len();
                    let start_line = scroll_offset.min(total_lines);
                    let end_line = (start_line + available_height).min(total_lines);

                    if start_line < total_lines {
                        let display_lines = &table_lines[start_line..end_line];
                        for line in display_lines {
                            println!("{}\r", line);
                        }
                    }

                    // Clear bottom of screen
                    execute!(io::stdout(), Clear(ClearType::FromCursorDown)).ok();

                    // Display controls at bottom
                    execute!(
                        io::stdout(),
                        cursor::MoveTo(0, terminal_height - 2),
                        SetForegroundColor(Color::Green),
                        Print("Controls: ↑↓/jk=scroll | n/p=page | Page Up/Down | Home/End | q=quit | h=help"),
                        ResetColor
                    ).ok();

                    io::stdout().flush().ok();

                    // Handle keyboard input
                    if let Ok(Event::Key(KeyEvent {
                        code,
                        kind: KeyEventKind::Press,
                        ..
                    })) = event::read()
                    {
                        match code {
                            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,
                            KeyCode::Up | KeyCode::Char('k') => {
                                scroll_offset = scroll_offset.saturating_sub(1);
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if scroll_offset + available_height < total_lines {
                                    scroll_offset += 1;
                                }
                            }
                            KeyCode::PageUp => {
                                scroll_offset = scroll_offset.saturating_sub(available_height);
                            }
                            KeyCode::PageDown => {
                                scroll_offset = (scroll_offset + available_height)
                                    .min(total_lines.saturating_sub(available_height));
                            }
                            KeyCode::Home => scroll_offset = 0,
                            KeyCode::End => {
                                scroll_offset = total_lines.saturating_sub(available_height);
                            }
                            KeyCode::Char('h') | KeyCode::Char('H') => {
                                execute!(
                                    io::stdout(),
                                    Clear(ClearType::All),
                                    cursor::MoveTo(0, 0),
                                    SetForegroundColor(Color::Cyan),
                                    Print("Dashboard Details - Keyboard Navigation Help"),
                                    ResetColor,
                                    Print("\r\n\r\n"),
                                    Print("Scroll Controls:\r\n"),
                                    Print("  ↑, k        : Scroll up (1 line)\r\n"),
                                    Print("  ↓, j        : Scroll down (1 line)\r\n"),
                                    Print("  Page Up     : Scroll up (page)\r\n"),
                                    Print("  Page Down   : Scroll down (page)\r\n"),
                                    Print("  Home        : Top\r\n"),
                                    Print("  End         : Bottom\r\n"),
                                    Print("\r\n"),
                                    Print("Other Controls:\r\n"),
                                    Print("  q, Q, Esc  : Quit\r\n"),
                                    Print("  h, H       : Show this help\r\n"),
                                    Print("\r\n"),
                                    SetForegroundColor(Color::Yellow),
                                    Print("Press any key to continue..."),
                                    ResetColor
                                )
                                .ok();
                                io::stdout().flush().ok();
                                event::read().ok();
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(_) => {
                // Fallback to simple display
                println!("Warning: Could not enable full-screen mode, falling back to simple mode");
                let table_display = crate::display::table::TableDisplay::new();
                let table_content = table_display.render_dashboard_details(dashboard)?;
                println!("{}", table_content);
            }
        }

        Ok(())
    }

    /// Display dashboard cards with interactive features and pagination
    pub async fn display_dashboard_cards_interactive(
        &self,
        cards: &[DashboardCard],
        dashboard_id: u32,
        page_size: usize,
    ) -> Result<(), AppError> {
        // Implement pagination similar to dashboard_list_pagination
        let total_cards = cards.len();
        let total_pages = total_cards.div_ceil(page_size); // Ceiling division
        let mut current_page = 1;

        // Enable raw mode for keyboard handling
        match terminal::enable_raw_mode() {
            Ok(_) => {
                let mut scroll_offset = 0;
                let (_, terminal_height) = terminal::size().unwrap_or((80, 24));
                let header_height = 4; // Title + info + blank line
                let footer_height = 2; // Controls + status
                let available_height =
                    (terminal_height as usize).saturating_sub(header_height + footer_height);

                loop {
                    // Calculate current page data
                    let start_idx = (current_page - 1) * page_size;
                    let end_idx = (start_idx + page_size).min(total_cards);
                    let current_cards = &cards[start_idx..end_idx];

                    // Generate table for current page
                    let table_display = crate::display::table::TableDisplay::new();
                    let table_content = table_display.render_dashboard_cards(current_cards)?;
                    let table_lines: Vec<String> =
                        table_content.lines().map(|s| s.to_string()).collect();

                    // Clear entire screen
                    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();

                    // Display header (fixed)
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Cyan),
                        Print(format!("Dashboard Cards - Dashboard {}", dashboard_id)),
                        ResetColor,
                        Print("\r\n"),
                        SetForegroundColor(Color::Yellow),
                        Print(format!(
                            "Page {}/{} | Showing cards {}-{} of {} total | Page size: {}",
                            current_page,
                            total_pages,
                            start_idx + 1,
                            end_idx,
                            total_cards,
                            page_size
                        )),
                        ResetColor,
                        Print("\r\n\r\n")
                    )
                    .ok();

                    // Display content within scroll range
                    let total_lines = table_lines.len();
                    let start_line = scroll_offset.min(total_lines);
                    let end_line = (start_line + available_height).min(total_lines);

                    if start_line < total_lines {
                        let display_lines = &table_lines[start_line..end_line];
                        for line in display_lines {
                            println!("{}\r", line);
                        }
                    }

                    // Clear bottom of screen
                    execute!(io::stdout(), Clear(ClearType::FromCursorDown)).ok();

                    // Display controls at bottom
                    execute!(
                        io::stdout(),
                        cursor::MoveTo(0, terminal_height - 2),
                        SetForegroundColor(Color::Green),
                        Print("Controls: ↑↓/jk=scroll | n/p=page | Page Up/Down | Home/End | q=quit | h=help"),
                        ResetColor
                    ).ok();

                    io::stdout().flush().ok();

                    // Handle keyboard input
                    if let Ok(Event::Key(KeyEvent {
                        code,
                        kind: KeyEventKind::Press,
                        ..
                    })) = event::read()
                    {
                        match code {
                            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,
                            KeyCode::Up | KeyCode::Char('k') => {
                                scroll_offset = scroll_offset.saturating_sub(1);
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if scroll_offset + available_height < total_lines {
                                    scroll_offset += 1;
                                }
                            }
                            KeyCode::PageUp => {
                                scroll_offset = scroll_offset.saturating_sub(available_height);
                            }
                            KeyCode::PageDown => {
                                scroll_offset = (scroll_offset + available_height)
                                    .min(total_lines.saturating_sub(available_height));
                            }
                            KeyCode::Home => {
                                current_page = 1;
                                scroll_offset = 0;
                            }
                            KeyCode::End => {
                                current_page = total_pages.max(1);
                                scroll_offset = 0;
                            }
                            // Page navigation
                            KeyCode::Char('n') | KeyCode::Char('N') => {
                                if current_page < total_pages {
                                    current_page += 1;
                                    scroll_offset = 0;
                                }
                            }
                            KeyCode::Char('p') | KeyCode::Char('P') => {
                                if current_page > 1 {
                                    current_page -= 1;
                                    scroll_offset = 0;
                                }
                            }
                            KeyCode::Char('h') | KeyCode::Char('H') => {
                                execute!(
                                    io::stdout(),
                                    Clear(ClearType::All),
                                    cursor::MoveTo(0, 0),
                                    SetForegroundColor(Color::Cyan),
                                    Print("Dashboard Cards - Keyboard Navigation Help"),
                                    ResetColor,
                                    Print("\r\n\r\n"),
                                    Print("Scroll Controls:\r\n"),
                                    Print("  ↑, k        : Scroll up (1 line)\r\n"),
                                    Print("  ↓, j        : Scroll down (1 line)\r\n"),
                                    Print("  Page Up     : Scroll up (page)\r\n"),
                                    Print("  Page Down   : Scroll down (page)\r\n"),
                                    Print("  Home        : Top\r\n"),
                                    Print("  End         : Bottom\r\n"),
                                    Print("\r\n"),
                                    Print("Other Controls:\r\n"),
                                    Print("  q, Q, Esc  : Quit\r\n"),
                                    Print("  h, H       : Show this help\r\n"),
                                    Print("\r\n"),
                                    SetForegroundColor(Color::Yellow),
                                    Print("Press any key to continue..."),
                                    ResetColor
                                )
                                .ok();
                                io::stdout().flush().ok();
                                event::read().ok();
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(_) => {
                // Fallback to simple display
                println!("Warning: Could not enable full-screen mode, falling back to simple mode");
                let table_display = crate::display::table::TableDisplay::new();
                let table_content = table_display.render_dashboard_cards(cards)?;
                println!("{}", table_content);
            }
        }

        Ok(())
    }

    /// Display collection list in fullscreen mode with proper terminal handling  
    /// This replaces the old display_collection_list_pagination with a stable implementation
    pub async fn display_collection_list_fullscreen(
        &self,
        collections: &[Collection],
        page_size: usize,
    ) -> Result<(), AppError> {
        use crossterm::terminal::{
            EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size,
        };
        use std::io::{self, Write};

        // RAII cleanup structures
        struct RawModeCleanup;
        impl Drop for RawModeCleanup {
            fn drop(&mut self) {
                let _ = disable_raw_mode();
            }
        }

        struct ScreenCleanup;
        impl Drop for ScreenCleanup {
            fn drop(&mut self) {
                let _ = execute!(io::stdout(), LeaveAlternateScreen);
            }
        }

        // Full screen mode - RAW mode + Alternate Screen + pagination
        match enable_raw_mode() {
            Ok(()) => {
                let _cleanup = RawModeCleanup;
                execute!(io::stdout(), EnterAlternateScreen).ok();
                let _screen_cleanup = ScreenCleanup;

                // Get terminal size for dynamic layout
                let (terminal_width, terminal_height) = size().unwrap_or((80, 24));

                // Pagination state
                let total_collections = collections.len();
                let total_pages = if total_collections == 0 {
                    1
                } else {
                    total_collections.div_ceil(page_size)
                };
                let mut current_page = 1;

                // Scroll state (for scrolling within table)
                let mut scroll_offset = 0;
                // Reserve 6 lines: header space (3 lines) + prompt space (3 lines)
                let available_height = terminal_height.saturating_sub(6) as usize;

                loop {
                    // Clear screen and reset cursor
                    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();

                    // Header with colored title
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Cyan),
                        Print("Collection List"),
                        ResetColor,
                        Print("\r\n"),
                        SetForegroundColor(Color::Yellow),
                        Print(format!(
                            "Total: {} collections | Page {} of {} | Showing {} items per page",
                            total_collections, current_page, total_pages, page_size
                        )),
                        ResetColor,
                        Print("\r\n\r\n")
                    )
                    .ok();

                    // Calculate appropriate column widths based on terminal size
                    let total_width = terminal_width as usize;
                    let available_width = total_width.saturating_sub(7); // Border chars

                    // Distribute width: 8% ID, 35% Name, 45% Description, 12% Type
                    let id_width = (available_width * 8 / 100).clamp(4, 8);
                    let name_width = (available_width * 35 / 100).clamp(10, 25);
                    let desc_width = (available_width * 45 / 100).clamp(15, 35);
                    let type_width =
                        available_width.saturating_sub(id_width + name_width + desc_width);

                    // Dynamic table header
                    let top_border = format!("┌{:─<id$}┬{:─<name$}┬{:─<desc$}┬{:─<type$}┐", "", "", "", "", 
                                            id = id_width, name = name_width, desc = desc_width, type = type_width);
                    let header_row = format!("│{:^id$}│{:^name$}│{:^desc$}│{:^type$}│", 
                                            "ID", "Name", "Description", "Type",
                                            id = id_width, name = name_width, desc = desc_width, type = type_width);
                    let separator = format!("├{:─<id$}┼{:─<name$}┼{:─<desc$}┼{:─<type$}┤", "", "", "", "",
                                          id = id_width, name = name_width, desc = desc_width, type = type_width);

                    execute!(
                        io::stdout(),
                        Print(&top_border),
                        Print("\r\n"),
                        Print(&header_row),
                        Print("\r\n"),
                        Print(&separator),
                        Print("\r\n")
                    )
                    .ok();

                    // Get current page data
                    let start_idx = (current_page - 1) * page_size;
                    let end_idx = (start_idx + page_size).min(total_collections);
                    let page_collections = if start_idx < total_collections {
                        &collections[start_idx..end_idx]
                    } else {
                        &[]
                    };

                    // Display collection rows
                    let table_lines: Vec<String> = page_collections.iter().map(|collection| {
                        let id_str = collection.id.map_or("root".to_string(), |id| id.to_string());
                        let name = self.truncate_string(&collection.name, name_width);
                        let description = self.truncate_string("", desc_width); // Collections don't have descriptions
                        let collection_type = if collection.id.is_none() { "Root" } else { "Collection" };
                        format!("│{:id$}│{:name$}│{:desc$}│{:type$}│", 
                                id_str, name, description, collection_type,
                                id = id_width, name = name_width, desc = desc_width, type = type_width)
                    }).collect();

                    // Display table lines with scrolling
                    let total_lines = table_lines.len();
                    let start_line = scroll_offset.min(total_lines);
                    let end_line = (start_line + available_height).min(total_lines);

                    if start_line < total_lines {
                        let display_lines = &table_lines[start_line..end_line];
                        for line in display_lines {
                            execute!(io::stdout(), Print(line), Print("\r\n")).ok();
                        }
                    }

                    // Table bottom border
                    let bottom_border = format!("└{:─<id$}┴{:─<name$}┴{:─<desc$}┴{:─<type$}┘", "", "", "", "",
                                               id = id_width, name = name_width, desc = desc_width, type = type_width);
                    execute!(io::stdout(), Print(&bottom_border), Print("\r\n\r\n")).ok();

                    // Control instructions
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Green),
                        Print("Controls: [q]uit | [n]ext page | [p]revious page | [h]elp"),
                        ResetColor
                    )
                    .ok();

                    io::stdout().flush().ok();

                    // Handle input
                    if let Event::Key(KeyEvent { code, kind, .. }) =
                        event::read().unwrap_or(Event::Key(KeyEvent::from(KeyCode::Char('q'))))
                    {
                        if kind == KeyEventKind::Press {
                            match code {
                                KeyCode::Char('q') | KeyCode::Esc => break,
                                KeyCode::Char('n') | KeyCode::Right => {
                                    if current_page < total_pages {
                                        current_page += 1;
                                        scroll_offset = 0;
                                    }
                                }
                                KeyCode::Char('p') | KeyCode::Left => {
                                    if current_page > 1 {
                                        current_page -= 1;
                                        scroll_offset = 0;
                                    }
                                }
                                KeyCode::Char('j') | KeyCode::Down => {
                                    if start_line + available_height < total_lines {
                                        scroll_offset += 1;
                                    }
                                }
                                KeyCode::Char('k') | KeyCode::Up => {
                                    scroll_offset = scroll_offset.saturating_sub(1);
                                }
                                KeyCode::Home => {
                                    current_page = 1;
                                    scroll_offset = 0;
                                }
                                KeyCode::End => {
                                    current_page = total_pages;
                                    scroll_offset = 0;
                                }
                                KeyCode::Char('h') => {
                                    self.show_collection_list_help().await?;
                                }
                                _ => {} // Ignore other keys
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // Fallback: Simple output if RAW mode fails
                println!("Collection List ({} collections):", collections.len());
                for (i, collection) in collections.iter().enumerate() {
                    let id_str = collection
                        .id
                        .map_or("root".to_string(), |id| id.to_string());
                    let collection_type = if collection.id.is_none() {
                        "Root"
                    } else {
                        "Collection"
                    };
                    println!(
                        "{:3}. ID: {:8} | Name: {:25} | Type: {}",
                        i + 1,
                        id_str,
                        self.truncate_string(&collection.name, 25),
                        collection_type
                    );
                }
            }
        }

        Ok(())
    }

    /// Help display for collection list
    async fn show_collection_list_help(&self) -> Result<(), AppError> {
        execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();

        execute!(
            io::stdout(),
            SetForegroundColor(Color::Cyan),
            Print("Collection List - Help"),
            ResetColor,
            Print("\r\n\r\n"),
            Print("Available Commands:\r\n"),
            Print("  q - Quit and return to previous screen\r\n"),
            Print("  n - Next page\r\n"),
            Print("  p - Previous page\r\n"),
            Print("  j/↓ - Scroll down within current page\r\n"),
            Print("  k/↑ - Scroll up within current page\r\n"),
            Print("  Home - Go to first page\r\n"),
            Print("  End - Go to last page\r\n"),
            Print("  h - Show this help\r\n"),
            Print("  ESC - Same as 'q'\r\n"),
            Print("\r\n"),
            Print("This screen shows all collections in your Metabase instance.\r\n"),
            Print("Collections are used to organize questions and dashboards.\r\n"),
            Print("The root collection contains items not placed in other collections.\r\n"),
            Print("\r\n"),
            SetForegroundColor(Color::Yellow),
            Print("Press any key to return..."),
            ResetColor
        )
        .ok();

        io::stdout().flush().ok();
        event::read().ok(); // Wait for any key press

        Ok(())
    }

    /// Display collection details in fullscreen mode with proper terminal handling
    /// This replaces the old display_collection_details_interactive with a stable implementation
    pub async fn display_collection_details_fullscreen(
        &self,
        collection: &CollectionDetail,
    ) -> Result<(), AppError> {
        use crossterm::terminal::{
            EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size,
        };
        use std::io::{self, Write};

        // RAII cleanup structures
        struct RawModeCleanup;
        impl Drop for RawModeCleanup {
            fn drop(&mut self) {
                let _ = disable_raw_mode();
            }
        }

        struct ScreenCleanup;
        impl Drop for ScreenCleanup {
            fn drop(&mut self) {
                let _ = execute!(io::stdout(), LeaveAlternateScreen);
            }
        }

        // Full screen mode - RAW mode + Alternate Screen
        match enable_raw_mode() {
            Ok(()) => {
                let _cleanup = RawModeCleanup;
                execute!(io::stdout(), EnterAlternateScreen).ok();
                let _screen_cleanup = ScreenCleanup;

                // Get terminal size for dynamic width calculation
                let (terminal_width, _terminal_height) = size().unwrap_or((80, 24));

                // Calculate appropriate column widths based on terminal size
                let total_width = terminal_width as usize;
                let border_width = 4; // "│ " + " │"
                let separator_width = 1; // "│"
                let available_width = total_width.saturating_sub(border_width + separator_width);

                // Distribute width: 30% for field name, 70% for value
                let field_width = (available_width * 30 / 100).clamp(8, 15);
                let value_width = available_width.saturating_sub(field_width);

                loop {
                    // Clear screen and reset cursor
                    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();

                    // Header with colored title
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Cyan),
                        Print("Collection Details"),
                        ResetColor,
                        Print("\r\n"),
                        SetForegroundColor(Color::Yellow),
                        Print(format!(
                            "ID: {} | Name: {}",
                            collection
                                .id
                                .map_or("root".to_string(), |id| id.to_string()),
                            collection.name
                        )),
                        ResetColor,
                        Print("\r\n\r\n")
                    )
                    .ok();

                    // Dynamic table borders
                    let top_border = format!(
                        "┌{:─<width$}┬{:─<vwidth$}┐",
                        "",
                        "",
                        width = field_width,
                        vwidth = value_width
                    );
                    let _separator = format!(
                        "├{:─<width$}┼{:─<vwidth$}┤",
                        "",
                        "",
                        width = field_width,
                        vwidth = value_width
                    );
                    let bottom_border = format!(
                        "└{:─<width$}┴{:─<vwidth$}┘",
                        "",
                        "",
                        width = field_width,
                        vwidth = value_width
                    );

                    execute!(io::stdout(), Print(&top_border), Print("\r\n")).ok();

                    // Display fields using execute for consistency
                    let id_str = if let Some(id) = collection.id {
                        format!("{}", id)
                    } else {
                        "root".to_string()
                    };

                    // Helper closure to print table row
                    let print_row = |field: &str, value: &str| {
                        let truncated_field = self.truncate_string(field, field_width);
                        let truncated_value = self.truncate_string(value, value_width);
                        execute!(
                            io::stdout(),
                            Print(format!(
                                "│{:width$}│{:vwidth$}│\r\n",
                                truncated_field,
                                truncated_value,
                                width = field_width,
                                vwidth = value_width
                            ))
                        )
                        .ok();
                    };

                    print_row("ID", &id_str);
                    print_row("Name", &collection.name);

                    if let Some(description) = &collection.description {
                        print_row("Description", description);
                    }

                    if let Some(color) = &collection.color {
                        print_row("Color", color);
                    }

                    if let Some(parent_id) = collection.parent_id {
                        print_row("Parent ID", &parent_id.to_string());
                    }

                    if let Some(created_at) = &collection.created_at {
                        print_row("Created", created_at);
                    }

                    if let Some(updated_at) = &collection.updated_at {
                        print_row("Updated", updated_at);
                    }

                    execute!(io::stdout(), Print(&bottom_border), Print("\r\n\r\n")).ok();

                    // Control instructions
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Green),
                        Print("Controls: [q]uit | [h]elp"),
                        ResetColor
                    )
                    .ok();

                    io::stdout().flush().ok();

                    // Handle input
                    if let Event::Key(KeyEvent { code, kind, .. }) =
                        event::read().unwrap_or(Event::Key(KeyEvent::from(KeyCode::Char('q'))))
                    {
                        if kind == KeyEventKind::Press {
                            match code {
                                KeyCode::Char('q') | KeyCode::Esc => break,
                                KeyCode::Char('h') => {
                                    self.show_collection_details_help().await?;
                                }
                                _ => {} // Ignore other keys
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // Fallback: Simple output if RAW mode fails
                println!("Collection Details:");
                println!(
                    "ID: {}",
                    collection
                        .id
                        .map_or("root".to_string(), |id| id.to_string())
                );
                println!("Name: {}", collection.name);

                if let Some(description) = &collection.description {
                    println!("Description: {}", description);
                }

                if let Some(color) = &collection.color {
                    println!("Color: {}", color);
                }

                if let Some(parent_id) = collection.parent_id {
                    println!("Parent ID: {}", parent_id);
                }

                if let Some(created_at) = &collection.created_at {
                    println!("Created: {}", created_at);
                }

                if let Some(updated_at) = &collection.updated_at {
                    println!("Updated: {}", updated_at);
                }
            }
        }

        Ok(())
    }

    /// Help display for collection details
    async fn show_collection_details_help(&self) -> Result<(), AppError> {
        execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();

        execute!(
            io::stdout(),
            SetForegroundColor(Color::Cyan),
            Print("Collection Details - Help"),
            ResetColor,
            Print("\r\n\r\n"),
            Print("Available Commands:\r\n"),
            Print("  q - Quit and return to previous screen\r\n"),
            Print("  h - Show this help\r\n"),
            Print("  ESC - Same as 'q'\r\n"),
            Print("\r\n"),
            Print("This screen shows detailed information about a Metabase collection.\r\n"),
            Print("Collections are used to organize questions and dashboards.\r\n"),
            Print("\r\n"),
            SetForegroundColor(Color::Yellow),
            Print("Press any key to return..."),
            ResetColor
        )
        .ok();

        io::stdout().flush().ok();
        event::read().ok(); // Wait for any key press

        Ok(())
    }

    /// Display collection statistics interactively
    pub async fn display_collection_stats_interactive(
        &self,
        stats: &CollectionStats,
        collection_id: u32,
    ) -> Result<(), AppError> {
        // Try to enable RAW mode for full-screen display
        match terminal::enable_raw_mode() {
            Ok(_) => {
                loop {
                    // Clear screen and reset cursor
                    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();

                    // Print statistics
                    println!(
                        "Collection Statistics (ID: {}) | [q]uit | [h]elp",
                        collection_id
                    );
                    println!(
                        "┌──────────────────┬─────────────────────────────────────────────────────────────┐"
                    );
                    println!("│ Total Items      │ {:59} │", stats.item_count);
                    println!("│ Questions        │ {:59} │", stats.question_count);
                    println!("│ Dashboards       │ {:59} │", stats.dashboard_count);

                    if let Some(last_updated) = &stats.last_updated {
                        println!(
                            "│ Last Updated     │ {:59} │",
                            self.truncate_string(last_updated, 59)
                        );
                    }

                    println!(
                        "└──────────────────┴─────────────────────────────────────────────────────────────┘"
                    );

                    // Control instructions with color (similar to query results)
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Green),
                        Print("Controls: [q]uit | [h]elp"),
                        ResetColor
                    )
                    .ok();

                    io::stdout().flush().ok();

                    // Handle input
                    if let Event::Key(KeyEvent { code, kind, .. }) =
                        event::read().unwrap_or(Event::Key(KeyEvent::from(KeyCode::Char('q'))))
                    {
                        if kind == KeyEventKind::Press {
                            match code {
                                KeyCode::Char('q') | KeyCode::Esc => break,
                                KeyCode::Char('h') => {
                                    self.show_collection_details_help().await?;
                                }
                                _ => {} // Ignore other keys
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // Fallback when RAW mode fails
                println!("Warning: Could not enable full-screen mode, falling back to simple mode");
                println!("Collection Statistics (ID: {}):", collection_id);
                println!("  Total Items: {}", stats.item_count);
                println!("  Questions: {}", stats.question_count);
                println!("  Dashboards: {}", stats.dashboard_count);
                if let Some(last_updated) = &stats.last_updated {
                    println!("  Last Updated: {}", last_updated);
                }
            }
        }

        Ok(())
    }
}
