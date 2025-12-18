use crate::api::models::{
    Collection, CollectionDetail, CollectionStats, Dashboard, DashboardCard, QueryResult, Question,
};
use crate::error::AppError;
use crate::utils::text::{format_datetime, pad_to_width, truncate_text, wrap_text};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
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

    pub async fn display_dashboard_list_pagination(
        &self,
        dashboards: &[Dashboard],
        page_size: usize,
    ) -> Result<(), AppError> {
        match terminal::enable_raw_mode() {
            Ok(()) => {
                let _cleanup = RawModeCleanup;
                execute!(io::stdout(), EnterAlternateScreen).ok();
                let _screen_cleanup = ScreenCleanup;

                let (_, terminal_height) = terminal::size().unwrap_or((80, 24));
                let total = dashboards.len();
                let total_pages = total.div_ceil(page_size).max(1);
                let mut current_page = 1;
                let mut scroll_offset = 0;
                let available_height = terminal_height.saturating_sub(8) as usize;

                loop {
                    let start = (current_page - 1) * page_size;
                    let end = (start + page_size).min(total);
                    let page_items = if start < total {
                        &dashboards[start..end]
                    } else {
                        &[]
                    };

                    let mut table_lines = vec![
                        "┌──────┬─────────────────────────────────┬─────────────────────────────────┬──────────────────┬──────────────────┐".to_string(),
                        "│ ID   │ Name                            │ Description                     │ Collection       │ Updated          │".to_string(),
                        "├──────┼─────────────────────────────────┼─────────────────────────────────┼──────────────────┼──────────────────┤".to_string(),
                    ];

                    for d in page_items {
                        let name_w = wrap_text(&d.name, 31);
                        let desc_w = d
                            .description
                            .as_ref()
                            .map(|s| wrap_text(s, 31))
                            .unwrap_or_else(|| vec!["".into()]);
                        let coll_w = wrap_text(
                            &d.collection_id
                                .map(|id| format!("ID: {}", id))
                                .unwrap_or_else(|| "Personal".into()),
                            16,
                        );
                        let upd_w = wrap_text(&format_datetime(&d.updated_at), 16);
                        let max_lines = name_w
                            .len()
                            .max(desc_w.len())
                            .max(coll_w.len())
                            .max(upd_w.len());
                        for i in 0..max_lines {
                            let empty = String::new();
                            table_lines.push(format!(
                                "│ {:>4} │ {:31} │ {:31} │ {:16} │ {:16} │",
                                if i == 0 {
                                    d.id.to_string()
                                } else {
                                    String::new()
                                },
                                pad_to_width(name_w.get(i).unwrap_or(&empty), 31),
                                pad_to_width(desc_w.get(i).unwrap_or(&empty), 31),
                                pad_to_width(coll_w.get(i).unwrap_or(&empty), 16),
                                pad_to_width(upd_w.get(i).unwrap_or(&empty), 16)
                            ));
                        }
                    }
                    table_lines.push("└──────┴─────────────────────────────────┴─────────────────────────────────┴──────────────────┴──────────────────┘".to_string());

                    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Cyan),
                        Print("Dashboard List"),
                        ResetColor,
                        Print("\r\n"),
                        SetForegroundColor(Color::Yellow),
                        Print(format!(
                            "Page {}/{} | Showing {}-{} of {} | page_size: {}",
                            current_page,
                            total_pages,
                            start + 1,
                            end,
                            total,
                            page_size
                        )),
                        ResetColor,
                        Print("\r\n\r\n")
                    )
                    .ok();

                    let lines_ref: Vec<&str> = table_lines.iter().map(|s| s.as_str()).collect();
                    display_table_lines(&lines_ref, scroll_offset, available_height);
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
                                scroll_offset = scroll_offset.saturating_sub(available_height / 2);
                            }
                            KeyAction::PageDown => {
                                let max = table_lines.len().saturating_sub(available_height);
                                scroll_offset = (scroll_offset + available_height / 2).min(max);
                            }
                            KeyAction::NextPage if current_page < total_pages => {
                                current_page += 1;
                                scroll_offset = 0;
                            }
                            KeyAction::PrevPage if current_page > 1 => {
                                current_page -= 1;
                                scroll_offset = 0;
                            }
                            KeyAction::Home => scroll_offset = 0,
                            KeyAction::End => {
                                scroll_offset = table_lines.len().saturating_sub(available_height);
                            }
                            KeyAction::Help => {
                                show_standard_help("Dashboard List - Keyboard Navigation Help")
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(_) => {
                println!("Warning: Could not enable full-screen mode, falling back to simple mode");
                println!("Dashboard List ({} found):", dashboards.len());
                for d in dashboards {
                    println!(
                        "  ID: {}, Name: {}, Description: {:?}",
                        d.id, d.name, d.description
                    );
                }
            }
        }
        Ok(())
    }

    pub async fn display_dashboard_details_interactive(
        &self,
        dashboard: &Dashboard,
    ) -> Result<(), AppError> {
        let table_display = crate::display::table::TableDisplay::new();
        let table_content = table_display.render_dashboard_details(dashboard)?;
        let table_lines: Vec<String> = table_content.lines().map(|s| s.to_string()).collect();

        match terminal::enable_raw_mode() {
            Ok(_) => {
                let _cleanup = RawModeCleanup;
                let mut scroll_offset = 0;
                let (_, terminal_height) = terminal::size().unwrap_or((80, 24));
                let available_height = (terminal_height as usize).saturating_sub(6);

                loop {
                    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();
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

                    let total = table_lines.len();
                    let start = scroll_offset.min(total);
                    let end = (start + available_height).min(total);
                    if start < total {
                        for line in &table_lines[start..end] {
                            println!("{}\r", line);
                        }
                    }

                    execute!(io::stdout(), Clear(ClearType::FromCursorDown)).ok();
                    display_controls(terminal_height);
                    io::stdout().flush().ok();

                    if let Ok(Event::Key(KeyEvent {
                        code,
                        kind: KeyEventKind::Press,
                        modifiers,
                        ..
                    })) = event::read()
                    {
                        match parse_key_event(code, modifiers) {
                            KeyAction::Quit => break,
                            KeyAction::ScrollUp => scroll_offset = scroll_offset.saturating_sub(1),
                            KeyAction::ScrollDown if scroll_offset + available_height < total => {
                                scroll_offset += 1;
                            }
                            KeyAction::PageUp => {
                                scroll_offset = scroll_offset.saturating_sub(available_height);
                            }
                            KeyAction::PageDown => {
                                scroll_offset = (scroll_offset + available_height)
                                    .min(total.saturating_sub(available_height));
                            }
                            KeyAction::Home => scroll_offset = 0,
                            KeyAction::End => {
                                scroll_offset = total.saturating_sub(available_height)
                            }
                            KeyAction::Help => {
                                show_standard_help("Dashboard Details - Keyboard Navigation Help")
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(_) => {
                println!("Warning: Could not enable full-screen mode, falling back to simple mode");
                println!("{}", table_content);
            }
        }
        Ok(())
    }

    pub async fn display_dashboard_cards_interactive(
        &self,
        cards: &[DashboardCard],
        dashboard_id: u32,
        page_size: usize,
    ) -> Result<(), AppError> {
        let total = cards.len();
        let total_pages = total.div_ceil(page_size).max(1);
        let mut current_page = 1;

        match terminal::enable_raw_mode() {
            Ok(_) => {
                let _cleanup = RawModeCleanup;
                let mut scroll_offset = 0;
                let (_, terminal_height) = terminal::size().unwrap_or((80, 24));
                let available_height = (terminal_height as usize).saturating_sub(6);

                loop {
                    let start = (current_page - 1) * page_size;
                    let end = (start + page_size).min(total);
                    let current_cards = cards.get(start..end).unwrap_or(&[]);

                    let table_display = crate::display::table::TableDisplay::new();
                    let table_content = table_display.render_dashboard_cards(current_cards)?;
                    let table_lines: Vec<String> =
                        table_content.lines().map(|s| s.to_string()).collect();

                    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Cyan),
                        Print(format!("Dashboard Cards - Dashboard {}", dashboard_id)),
                        ResetColor,
                        Print("\r\n"),
                        SetForegroundColor(Color::Yellow),
                        Print(format!(
                            "Page {}/{} | Showing {}-{} of {} | page_size: {}",
                            current_page,
                            total_pages,
                            start + 1,
                            end,
                            total,
                            page_size
                        )),
                        ResetColor,
                        Print("\r\n\r\n")
                    )
                    .ok();

                    let total_lines = table_lines.len();
                    let start_line = scroll_offset.min(total_lines);
                    let end_line = (start_line + available_height).min(total_lines);
                    if start_line < total_lines {
                        for line in &table_lines[start_line..end_line] {
                            println!("{}\r", line);
                        }
                    }

                    execute!(io::stdout(), Clear(ClearType::FromCursorDown)).ok();
                    display_controls(terminal_height);
                    io::stdout().flush().ok();

                    if let Ok(Event::Key(KeyEvent {
                        code,
                        kind: KeyEventKind::Press,
                        modifiers,
                        ..
                    })) = event::read()
                    {
                        match parse_key_event(code, modifiers) {
                            KeyAction::Quit => break,
                            KeyAction::ScrollUp => scroll_offset = scroll_offset.saturating_sub(1),
                            KeyAction::ScrollDown
                                if scroll_offset + available_height < total_lines =>
                            {
                                scroll_offset += 1;
                            }
                            KeyAction::PageUp => {
                                scroll_offset = scroll_offset.saturating_sub(available_height);
                            }
                            KeyAction::PageDown => {
                                scroll_offset = (scroll_offset + available_height)
                                    .min(total_lines.saturating_sub(available_height));
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
                                show_standard_help("Dashboard Cards - Keyboard Navigation Help")
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(_) => {
                println!("Warning: Could not enable full-screen mode, falling back to simple mode");
                let table_display = crate::display::table::TableDisplay::new();
                println!("{}", table_display.render_dashboard_cards(cards)?);
            }
        }
        Ok(())
    }

    pub async fn display_collection_list_fullscreen(
        &self,
        collections: &[Collection],
        page_size: usize,
    ) -> Result<(), AppError> {
        match terminal::enable_raw_mode() {
            Ok(()) => {
                let _cleanup = RawModeCleanup;
                execute!(io::stdout(), EnterAlternateScreen).ok();
                let _screen_cleanup = ScreenCleanup;

                let (terminal_width, terminal_height) = terminal::size().unwrap_or((80, 24));
                let total = collections.len();
                let total_pages = total.div_ceil(page_size).max(1);
                let mut current_page = 1;
                let mut scroll_offset = 0;
                let available_height = terminal_height.saturating_sub(6) as usize;

                let available_width = (terminal_width as usize).saturating_sub(7);
                let id_w = (available_width * 8 / 100).clamp(4, 8);
                let name_w = (available_width * 35 / 100).clamp(10, 25);
                let desc_w = (available_width * 45 / 100).clamp(15, 35);
                let type_w = available_width.saturating_sub(id_w + name_w + desc_w);

                let top = format!(
                    "┌{:─<id$}┬{:─<name$}┬{:─<desc$}┬{:─<t$}┐",
                    "",
                    "",
                    "",
                    "",
                    id = id_w,
                    name = name_w,
                    desc = desc_w,
                    t = type_w
                );
                let header = format!(
                    "│{:^id$}│{:^name$}│{:^desc$}│{:^t$}│",
                    "ID",
                    "Name",
                    "Description",
                    "Type",
                    id = id_w,
                    name = name_w,
                    desc = desc_w,
                    t = type_w
                );
                let sep = format!(
                    "├{:─<id$}┼{:─<name$}┼{:─<desc$}┼{:─<t$}┤",
                    "",
                    "",
                    "",
                    "",
                    id = id_w,
                    name = name_w,
                    desc = desc_w,
                    t = type_w
                );
                let bottom = format!(
                    "└{:─<id$}┴{:─<name$}┴{:─<desc$}┴{:─<t$}┘",
                    "",
                    "",
                    "",
                    "",
                    id = id_w,
                    name = name_w,
                    desc = desc_w,
                    t = type_w
                );

                loop {
                    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Cyan),
                        Print("Collection List"),
                        ResetColor,
                        Print("\r\n"),
                        SetForegroundColor(Color::Yellow),
                        Print(format!(
                            "Total: {} | Page {} of {} | page_size: {}",
                            total, current_page, total_pages, page_size
                        )),
                        ResetColor,
                        Print("\r\n\r\n"),
                        Print(&top),
                        Print("\r\n"),
                        Print(&header),
                        Print("\r\n"),
                        Print(&sep),
                        Print("\r\n")
                    )
                    .ok();

                    let start = (current_page - 1) * page_size;
                    let end = (start + page_size).min(total);
                    let page_items = if start < total {
                        &collections[start..end]
                    } else {
                        &[]
                    };

                    let table_lines: Vec<String> = page_items
                        .iter()
                        .map(|c| {
                            let id_str = c.id.map_or("root".into(), |id| id.to_string());
                            let ctype = if c.id.is_none() { "Root" } else { "Collection" };
                            format!(
                                "│{:id$}│{:name$}│{:desc$}│{:t$}│",
                                id_str,
                                truncate_text(&c.name, name_w),
                                truncate_text("", desc_w),
                                ctype,
                                id = id_w,
                                name = name_w,
                                desc = desc_w,
                                t = type_w
                            )
                        })
                        .collect();

                    let lines_len = table_lines.len();
                    let start_line = scroll_offset.min(lines_len);
                    let end_line = (start_line + available_height).min(lines_len);
                    if start_line < lines_len {
                        for line in &table_lines[start_line..end_line] {
                            execute!(io::stdout(), Print(line), Print("\r\n")).ok();
                        }
                    }

                    execute!(io::stdout(), Print(&bottom), Print("\r\n\r\n")).ok();
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Green),
                        Print("Controls: [q]uit | [n]ext | [p]rev | [h]elp"),
                        ResetColor
                    )
                    .ok();
                    io::stdout().flush().ok();

                    if let Ok(Event::Key(KeyEvent {
                        code,
                        kind: KeyEventKind::Press,
                        modifiers,
                        ..
                    })) = event::read()
                    {
                        match parse_key_event(code, modifiers) {
                            KeyAction::Quit => break,
                            KeyAction::NextPage if current_page < total_pages => {
                                current_page += 1;
                                scroll_offset = 0;
                            }
                            KeyAction::PrevPage if current_page > 1 => {
                                current_page -= 1;
                                scroll_offset = 0;
                            }
                            KeyAction::ScrollDown if start_line + available_height < lines_len => {
                                scroll_offset += 1;
                            }
                            KeyAction::ScrollUp => scroll_offset = scroll_offset.saturating_sub(1),
                            KeyAction::Home => {
                                current_page = 1;
                                scroll_offset = 0;
                            }
                            KeyAction::End => {
                                current_page = total_pages;
                                scroll_offset = 0;
                            }
                            KeyAction::Help => {
                                self.show_simple_help(
                                    "Collection List - Help",
                                    "This screen shows all collections.",
                                )
                                .await?
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(_) => {
                println!("Collection List ({} collections):", collections.len());
                for (i, c) in collections.iter().enumerate() {
                    let id_str = c.id.map_or("root".into(), |id| id.to_string());
                    let ctype = if c.id.is_none() { "Root" } else { "Collection" };
                    println!(
                        "{:3}. ID: {:8} | Name: {:25} | Type: {}",
                        i + 1,
                        id_str,
                        truncate_text(&c.name, 25),
                        ctype
                    );
                }
            }
        }
        Ok(())
    }

    pub async fn display_collection_details_fullscreen(
        &self,
        collection: &CollectionDetail,
    ) -> Result<(), AppError> {
        match terminal::enable_raw_mode() {
            Ok(()) => {
                let _cleanup = RawModeCleanup;
                execute!(io::stdout(), EnterAlternateScreen).ok();
                let _screen_cleanup = ScreenCleanup;

                let (terminal_width, _) = terminal::size().unwrap_or((80, 24));
                let available_width = (terminal_width as usize).saturating_sub(5);
                let field_w = (available_width * 30 / 100).clamp(8, 15);
                let value_w = available_width.saturating_sub(field_w);

                let top = format!("┌{:─<fw$}┬{:─<vw$}┐", "", "", fw = field_w, vw = value_w);
                let bottom = format!("└{:─<fw$}┴{:─<vw$}┘", "", "", fw = field_w, vw = value_w);

                loop {
                    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Cyan),
                        Print("Collection Details"),
                        ResetColor,
                        Print("\r\n"),
                        SetForegroundColor(Color::Yellow),
                        Print(format!(
                            "ID: {} | Name: {}",
                            collection.id.map_or("root".into(), |id| id.to_string()),
                            collection.name
                        )),
                        ResetColor,
                        Print("\r\n\r\n"),
                        Print(&top),
                        Print("\r\n")
                    )
                    .ok();

                    let print_row = |f: &str, v: &str| {
                        execute!(
                            io::stdout(),
                            Print(format!(
                                "│{:fw$}│{:vw$}│\r\n",
                                truncate_text(f, field_w),
                                truncate_text(v, value_w),
                                fw = field_w,
                                vw = value_w
                            ))
                        )
                        .ok();
                    };

                    print_row(
                        "ID",
                        &collection.id.map_or("root".into(), |id| id.to_string()),
                    );
                    print_row("Name", &collection.name);
                    if let Some(d) = &collection.description {
                        print_row("Description", d);
                    }
                    if let Some(c) = &collection.color {
                        print_row("Color", c);
                    }
                    if let Some(p) = collection.parent_id {
                        print_row("Parent ID", &p.to_string());
                    }
                    if let Some(c) = &collection.created_at {
                        print_row("Created", c);
                    }
                    if let Some(u) = &collection.updated_at {
                        print_row("Updated", u);
                    }

                    execute!(io::stdout(), Print(&bottom), Print("\r\n\r\n")).ok();
                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Green),
                        Print("Controls: [q]uit | [h]elp"),
                        ResetColor
                    )
                    .ok();
                    io::stdout().flush().ok();

                    if let Ok(Event::Key(KeyEvent {
                        code,
                        kind: KeyEventKind::Press,
                        modifiers,
                        ..
                    })) = event::read()
                    {
                        match parse_key_event(code, modifiers) {
                            KeyAction::Quit => break,
                            KeyAction::Help => {
                                self.show_simple_help(
                                    "Collection Details - Help",
                                    "Detailed info about this collection.",
                                )
                                .await?
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(_) => {
                println!("Collection Details:");
                println!(
                    "ID: {}",
                    collection.id.map_or("root".into(), |id| id.to_string())
                );
                println!("Name: {}", collection.name);
                if let Some(d) = &collection.description {
                    println!("Description: {}", d);
                }
                if let Some(c) = &collection.color {
                    println!("Color: {}", c);
                }
                if let Some(p) = collection.parent_id {
                    println!("Parent ID: {}", p);
                }
                if let Some(c) = &collection.created_at {
                    println!("Created: {}", c);
                }
                if let Some(u) = &collection.updated_at {
                    println!("Updated: {}", u);
                }
            }
        }
        Ok(())
    }

    pub async fn display_collection_stats_interactive(
        &self,
        stats: &CollectionStats,
        collection_id: u32,
    ) -> Result<(), AppError> {
        match terminal::enable_raw_mode() {
            Ok(_) => {
                let _cleanup = RawModeCleanup;
                loop {
                    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();
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
                    if let Some(u) = &stats.last_updated {
                        println!("│ Last Updated     │ {:59} │", truncate_text(u, 59));
                    }
                    println!(
                        "└──────────────────┴─────────────────────────────────────────────────────────────┘"
                    );

                    execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Green),
                        Print("Controls: [q]uit | [h]elp"),
                        ResetColor
                    )
                    .ok();
                    io::stdout().flush().ok();

                    if let Ok(Event::Key(KeyEvent {
                        code,
                        kind: KeyEventKind::Press,
                        modifiers,
                        ..
                    })) = event::read()
                    {
                        match parse_key_event(code, modifiers) {
                            KeyAction::Quit => break,
                            KeyAction::Help => {
                                self.show_simple_help(
                                    "Collection Stats - Help",
                                    "Statistics for this collection.",
                                )
                                .await?
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(_) => {
                println!("Warning: Could not enable full-screen mode, falling back to simple mode");
                println!("Collection Statistics (ID: {}):", collection_id);
                println!("  Total Items: {}", stats.item_count);
                println!("  Questions: {}", stats.question_count);
                println!("  Dashboards: {}", stats.dashboard_count);
                if let Some(u) = &stats.last_updated {
                    println!("  Last Updated: {}", u);
                }
            }
        }
        Ok(())
    }

    async fn show_simple_help(&self, title: &str, extra: &str) -> Result<(), AppError> {
        execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();
        execute!(
            io::stdout(),
            SetForegroundColor(Color::Cyan),
            Print(title),
            ResetColor,
            Print("\r\n\r\n"),
            Print("Commands:\r\n"),
            Print("  q/ESC - Quit\r\n"),
            Print("  n     - Next page\r\n"),
            Print("  p     - Previous page\r\n"),
            Print("  j/↓   - Scroll down\r\n"),
            Print("  k/↑   - Scroll up\r\n"),
            Print("  Home  - First page\r\n"),
            Print("  End   - Last page\r\n"),
            Print("  h     - Show this help\r\n"),
            Print("\r\n"),
            Print(extra),
            Print("\r\n\r\n"),
            SetForegroundColor(Color::Yellow),
            Print("Press any key to return..."),
            ResetColor
        )
        .ok();
        io::stdout().flush().ok();
        event::read().ok();
        Ok(())
    }
}
