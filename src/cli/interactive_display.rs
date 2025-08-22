use crate::api::models::{Question, QueryResult};
use crate::error::AppError;

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
        use crossterm::{
            cursor, event,
            event::{Event, KeyCode, KeyEvent, KeyModifiers},
            execute,
            style::{Color, Print, ResetColor, SetForegroundColor},
            terminal::{
                Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
                enable_raw_mode, size,
            },
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
        use crossterm::{
            cursor, event,
            event::{Event, KeyCode, KeyEvent, KeyModifiers},
            execute,
            style::{Color, Print, ResetColor, SetForegroundColor},
            terminal::{
                Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
                enable_raw_mode, size,
            },
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
}

