use crate::api::models::{QueryResult, Question};
use crate::error::AppError;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};

struct RawModeCleanup;
impl Drop for RawModeCleanup {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

struct ScreenCleanup;
impl Drop for ScreenCleanup {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

const CONTROLS_LINE: &str = "Controls: ↑↓/jk=scroll | n/p=page | Home/End | q=quit | h=help";

fn show_standard_help(title: &str) {
    execute!(
        io::stdout(),
        Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        SetForegroundColor(Color::Cyan),
        Print(title),
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

enum KeyAction {
    Quit,
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    NextPage,
    PrevPage,
    Home,
    End,
    Help,
    None,
}

fn parse_key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyAction {
    match code {
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => KeyAction::Quit,
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => KeyAction::Quit,
        KeyCode::Up | KeyCode::Char('k') => KeyAction::ScrollUp,
        KeyCode::Down | KeyCode::Char('j') => KeyAction::ScrollDown,
        KeyCode::PageUp => KeyAction::PageUp,
        KeyCode::PageDown => KeyAction::PageDown,
        KeyCode::Char('n') | KeyCode::Char('N') => KeyAction::NextPage,
        KeyCode::Char('p') | KeyCode::Char('P') => KeyAction::PrevPage,
        KeyCode::Home => KeyAction::Home,
        KeyCode::End => KeyAction::End,
        KeyCode::Char('h') | KeyCode::Char('H') => KeyAction::Help,
        _ => KeyAction::None,
    }
}

fn display_controls(terminal_height: u16) {
    execute!(
        io::stdout(),
        cursor::MoveTo(0, terminal_height - 2),
        SetForegroundColor(Color::Green),
        Print(CONTROLS_LINE),
        ResetColor
    )
    .ok();
}

fn display_table_lines(lines: &[&str], scroll_offset: usize, available_height: usize) {
    let total = lines.len();
    let start = scroll_offset.min(total);
    let end = (start + available_height).min(total);
    for i in start..end {
        if let Some(line) = lines.get(i) {
            println!("{}\r", line);
        }
    }
}

#[derive(Default)]
pub struct InteractiveDisplay;

impl InteractiveDisplay {
    pub fn new() -> Self {
        Self
    }

    pub async fn display_query_result_pagination(
        &self,
        result: &QueryResult,
        page_size: usize,
        initial_offset: Option<usize>,
        no_fullscreen: bool,
        question_id: u32,
        question_name: &str,
    ) -> Result<(), AppError> {
        if no_fullscreen {
            let display = crate::display::table::TableDisplay::new();
            println!("{}", display.render_query_result(result)?);
            return Ok(());
        }

        match terminal::enable_raw_mode() {
            Ok(()) => {
                let _cleanup = RawModeCleanup;
                execute!(io::stdout(), EnterAlternateScreen).ok();
                let _screen_cleanup = ScreenCleanup;

                let (_, terminal_height) = terminal::size().unwrap_or((80, 24));
                let total_rows = result.data.rows.len();
                let total_pages = total_rows.div_ceil(page_size).max(1);
                let mut current_page = 1;
                let mut scroll_offset = 0;
                let available_height = terminal_height.saturating_sub(8) as usize;
                let display = crate::display::table::TableDisplay::new();

                loop {
                    let start_row = (current_page - 1) * page_size;
                    let end_row = (start_row + page_size).min(total_rows);
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

                    let page_table_output = display.render_query_result(&page_result)?;
                    let table_lines: Vec<&str> = page_table_output.lines().collect();

                    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Cyan),
                        Print(format!("Question {}: {}", question_id, question_name)),
                        ResetColor,
                        Print("\r\n"),
                        SetForegroundColor(Color::Yellow),
                        Print(format!(
                            "Page {}/{} | Rows {}-{} of {} | offset: {} | page_size: {}",
                            current_page,
                            total_pages,
                            initial_offset.unwrap_or(0) + start_row + 1,
                            initial_offset.unwrap_or(0) + start_row + (end_row - start_row),
                            total_rows,
                            initial_offset.unwrap_or(0),
                            page_size
                        )),
                        ResetColor,
                        Print("\r\n\r\n")
                    )
                    .ok();

                    display_table_lines(&table_lines, scroll_offset, available_height);
                    execute!(io::stdout(), Clear(ClearType::FromCursorDown)).ok();
                    display_controls(terminal_height);
                    io::stdout().flush().ok();

                    if let Ok(Event::Key(KeyEvent {
                        code, modifiers, ..
                    })) = event::read()
                    {
                        match parse_key_event(code, modifiers) {
                            KeyAction::Quit => break,
                            KeyAction::ScrollUp => scroll_offset = scroll_offset.saturating_sub(1),
                            KeyAction::ScrollDown => {
                                let max = table_lines.len().saturating_sub(available_height);
                                scroll_offset = (scroll_offset + 1).min(max);
                            }
                            KeyAction::PageUp => {
                                scroll_offset = scroll_offset.saturating_sub(available_height);
                            }
                            KeyAction::PageDown => {
                                let max = table_lines.len().saturating_sub(available_height);
                                scroll_offset = (scroll_offset + available_height).min(max);
                            }
                            KeyAction::NextPage if current_page < total_pages => {
                                current_page += 1;
                                scroll_offset = 0;
                            }
                            KeyAction::PrevPage if current_page > 1 => {
                                current_page -= 1;
                                scroll_offset = 0;
                            }
                            KeyAction::Home => {
                                current_page = 1;
                                scroll_offset = 0;
                            }
                            KeyAction::End => {
                                current_page = total_pages;
                                scroll_offset = 0;
                            }
                            KeyAction::Help => show_standard_help("Keyboard Navigation Help"),
                            _ => {}
                        }
                    }
                }
            }
            Err(_) => {
                println!("Warning: Could not enable full-screen mode, falling back to simple mode");
                let display = crate::display::table::TableDisplay::new();
                println!("{}", display.render_query_result(result)?);
            }
        }
        Ok(())
    }

    pub async fn display_question_list_pagination(
        &self,
        questions: &[Question],
        page_size: usize,
    ) -> Result<(), AppError> {
        match terminal::enable_raw_mode() {
            Ok(()) => {
                let _cleanup = RawModeCleanup;
                execute!(io::stdout(), EnterAlternateScreen).ok();
                let _screen_cleanup = ScreenCleanup;

                let (_, terminal_height) = terminal::size().unwrap_or((80, 24));
                let total = questions.len();
                let total_pages = total.div_ceil(page_size).max(1);
                let mut current_page = 1;
                let mut scroll_offset = 0;
                let available_height = terminal_height.saturating_sub(6) as usize;
                let display = crate::display::table::TableDisplay::new();

                loop {
                    let start = (current_page - 1) * page_size;
                    let end = (start + page_size).min(total);
                    let page_items = if start < total {
                        &questions[start..end]
                    } else {
                        &[]
                    };

                    let table_output = display.render_question_list(page_items)?;
                    let table_lines: Vec<&str> = table_output.lines().collect();

                    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Cyan),
                        Print("Question List"),
                        ResetColor,
                        Print("\r\n"),
                        SetForegroundColor(Color::Yellow),
                        Print(format!(
                            "Page {}/{} | Showing {}-{} of {} | page_size: {}",
                            current_page,
                            total_pages,
                            start + 1,
                            start + page_items.len(),
                            total,
                            page_size
                        )),
                        ResetColor,
                        Print("\r\n\r\n")
                    )
                    .ok();

                    display_table_lines(&table_lines, scroll_offset, available_height);
                    execute!(io::stdout(), Clear(ClearType::FromCursorDown)).ok();
                    display_controls(terminal_height);
                    io::stdout().flush().ok();

                    if let Ok(Event::Key(KeyEvent {
                        code, modifiers, ..
                    })) = event::read()
                    {
                        match parse_key_event(code, modifiers) {
                            KeyAction::Quit => break,
                            KeyAction::ScrollUp => scroll_offset = scroll_offset.saturating_sub(1),
                            KeyAction::ScrollDown => {
                                let max = table_lines.len().saturating_sub(available_height);
                                scroll_offset = (scroll_offset + 1).min(max);
                            }
                            KeyAction::PageUp => {
                                scroll_offset = scroll_offset.saturating_sub(available_height);
                            }
                            KeyAction::PageDown => {
                                let max = table_lines.len().saturating_sub(available_height);
                                scroll_offset = (scroll_offset + available_height).min(max);
                            }
                            KeyAction::NextPage if current_page < total_pages => {
                                current_page += 1;
                                scroll_offset = 0;
                            }
                            KeyAction::PrevPage if current_page > 1 => {
                                current_page -= 1;
                                scroll_offset = 0;
                            }
                            KeyAction::Home => {
                                current_page = 1;
                                scroll_offset = 0;
                            }
                            KeyAction::End => {
                                current_page = total_pages;
                                scroll_offset = 0;
                            }
                            KeyAction::Help => {
                                show_standard_help("Question List - Keyboard Navigation Help")
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(_) => {
                println!("Warning: Could not enable full-screen mode, falling back to simple mode");
                let display = crate::display::table::TableDisplay::new();
                println!("{}", display.render_question_list(questions)?);
            }
        }
        Ok(())
    }
}
