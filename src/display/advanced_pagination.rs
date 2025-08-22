// Advanced pagination functionality based on reference implementation

// use crate::api::models::{QueryResult, QueryData};
use crate::error::AppError;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    // style::Print,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Keyboard input actions
#[derive(Debug, Clone, PartialEq)]
pub enum InputAction {
    NextPage,
    PreviousPage,
    Quit,
    ForceQuit,
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    Home,
    End,
    Help,
    Number(char),
    ConfirmNumber,
    CancelNumber,
    Invalid,
}

/// Advanced pagination manager
pub struct AdvancedPaginationManager {
    page_size: usize,
    current_page: usize,
    total_items: usize,
    scroll_offset: usize,
    terminal_height: u16,
    terminal_width: u16,
    number_input: String,
}

impl AdvancedPaginationManager {
    /// Create a new manager
    pub fn new(page_size: usize, total_items: usize) -> Result<Self, AppError> {
        let (width, height) = terminal::size().unwrap_or((80, 24)); // Safe fallback with default values

        // Ensure safe terminal size range
        let safe_width = width.clamp(40, 200);
        let safe_height = height.clamp(10, 100);

        Ok(Self {
            page_size,
            current_page: 0,
            total_items,
            scroll_offset: 0,
            terminal_height: safe_height,
            terminal_width: safe_width,
            number_input: String::new(),
        })
    }

    /// Handle interactive display
    pub fn handle_interactive_display<T, F>(
        &mut self,
        items: &[T],
        renderer: F,
    ) -> Result<(), AppError>
    where
        F: Fn(&[T]) -> String,
    {
        // Display all if not TTY (pipe output, etc.)
        if !atty::is(atty::Stream::Stdout) {
            println!("{}", renderer(items));
            return Ok(());
        }

        // Case of empty data
        if items.is_empty() {
            println!("No data available.");
            return Ok(());
        }

        // Use interactive mode even for small data
        // (Always respond to interactive mode requests)

        // Enter alternate screen
        execute!(io::stdout(), EnterAlternateScreen).map_err(|e| {
            AppError::Display(crate::error::DisplayError::TerminalOutput(format!(
                "Failed to enter alternate screen: {}",
                e
            )))
        })?;

        terminal::enable_raw_mode().map_err(|e| {
            AppError::Display(crate::error::DisplayError::TerminalOutput(format!(
                "Failed to enable raw mode: {}",
                e
            )))
        })?;

        let result = self.interactive_loop(items, renderer);

        // Restore original state
        terminal::disable_raw_mode().map_err(|e| {
            AppError::Display(crate::error::DisplayError::TerminalOutput(format!(
                "Failed to disable raw mode: {}",
                e
            )))
        })?;

        execute!(io::stdout(), LeaveAlternateScreen).map_err(|e| {
            AppError::Display(crate::error::DisplayError::TerminalOutput(format!(
                "Failed to leave alternate screen: {}",
                e
            )))
        })?;

        result
    }

    /// Handle interactive loop
    fn interactive_loop<T, F>(&mut self, items: &[T], renderer: F) -> Result<(), AppError>
    where
        F: Fn(&[T]) -> String,
    {
        loop {
            self.display_page(items, &renderer)?;

            match self.read_input()? {
                InputAction::Quit => break,
                InputAction::ForceQuit => break,
                InputAction::NextPage => {
                    self.next_page();
                }
                InputAction::PreviousPage => {
                    self.previous_page();
                }
                InputAction::ScrollUp => {
                    if self.scroll_offset > 0 {
                        self.scroll_offset -= 1;
                    }
                }
                InputAction::ScrollDown => {
                    let max_scroll = self.calculate_max_scroll(items, &renderer);
                    if self.scroll_offset < max_scroll {
                        self.scroll_offset += 1;
                    }
                }
                InputAction::PageUp => {
                    if self.scroll_offset >= 10 {
                        self.scroll_offset -= 10;
                    } else {
                        self.scroll_offset = 0;
                    }
                }
                InputAction::PageDown => {
                    let max_scroll = self.calculate_max_scroll(items, &renderer);
                    self.scroll_offset = (self.scroll_offset + 10).min(max_scroll);
                }
                InputAction::Home => {
                    self.current_page = 0;
                    self.scroll_offset = 0;
                }
                InputAction::End => {
                    let max_page = self.total_items.div_ceil(self.page_size);
                    self.current_page = max_page.saturating_sub(1);
                    self.scroll_offset = 0;
                }
                InputAction::Help => {
                    self.show_help()?;
                }
                InputAction::Number(digit) => {
                    self.number_input.push(digit);
                }
                InputAction::ConfirmNumber => {
                    if let Ok(page_num) = self.number_input.parse::<usize>() {
                        let max_page = self.total_items.div_ceil(self.page_size);
                        if page_num > 0 && page_num <= max_page {
                            self.current_page = page_num - 1;
                            self.scroll_offset = 0;
                        }
                    }
                    self.number_input.clear();
                }
                InputAction::CancelNumber => {
                    self.number_input.clear();
                }
                InputAction::Invalid => {
                    // Ignore invalid input
                }
            }
        }
        Ok(())
    }

    /// Read keyboard input and convert to actions
    fn read_input(&self) -> Result<InputAction, AppError> {
        match event::read().map_err(|e| {
            AppError::Display(crate::error::DisplayError::TerminalOutput(format!(
                "Failed to read key event: {}",
                e
            )))
        })? {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => {
                Ok(match (code, modifiers) {
                    // Exit
                    (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => InputAction::Quit,
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => InputAction::ForceQuit,

                    // Page navigation
                    (KeyCode::Char(']'), _) | (KeyCode::Right, _) | (KeyCode::Char('l'), _) => {
                        InputAction::NextPage
                    }
                    (KeyCode::Char('['), _) | (KeyCode::Left, _) | (KeyCode::Char('h'), _) => {
                        InputAction::PreviousPage
                    }

                    // Scroll
                    (KeyCode::Up, _) | (KeyCode::Char('k'), _) => InputAction::ScrollUp,
                    (KeyCode::Down, _) | (KeyCode::Char('j'), _) => InputAction::ScrollDown,
                    (KeyCode::PageUp, _) => InputAction::PageUp,
                    (KeyCode::PageDown, _) => InputAction::PageDown,

                    // Position jump
                    (KeyCode::Home, _) | (KeyCode::Char('g'), _) => InputAction::Home,
                    (KeyCode::End, _) | (KeyCode::Char('G'), _) => InputAction::End,

                    // Help
                    (KeyCode::Char('?'), _) => InputAction::Help,

                    // Number input
                    (KeyCode::Char(c), _) if c.is_ascii_digit() => InputAction::Number(c),
                    (KeyCode::Enter, _) => InputAction::ConfirmNumber,

                    _ => InputAction::Invalid,
                })
            }
            _ => Ok(InputAction::Invalid),
        }
    }

    /// Display current page
    fn display_page<T, F>(&self, items: &[T], renderer: &F) -> Result<(), AppError>
    where
        F: Fn(&[T]) -> String,
    {
        let start = self.current_page * self.page_size;
        let end = (start + self.page_size).min(items.len());

        if start >= items.len() {
            return Ok(());
        }

        let page_items = &items[start..end];

        // Clear screen
        execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).map_err(|e| {
            AppError::Display(crate::error::DisplayError::TerminalOutput(format!(
                "Failed to clear screen: {}",
                e
            )))
        })?;

        // Table display
        let table_content = renderer(page_items);
        let lines: Vec<&str> = table_content.lines().collect();

        // Calculate available display lines (excluding status lines)
        let available_lines = (self.terminal_height as usize).saturating_sub(3);

        // Apply scroll offset for display
        let display_lines = if self.scroll_offset < lines.len() {
            &lines[self.scroll_offset..]
        } else {
            &[]
        };

        // Safely display table rows
        for (i, line) in display_lines.iter().enumerate() {
            if i >= available_lines {
                break;
            }
            // Safely truncate lines exceeding terminal width considering Unicode display width
            let line_width = line.width();
            if line_width > self.terminal_width as usize {
                let target_width = self.terminal_width as usize;
                let mut truncated = String::new();
                let mut current_width = 0;

                for ch in line.chars() {
                    let char_width = ch.width().unwrap_or(0);
                    if current_width + char_width > target_width {
                        break;
                    }
                    truncated.push(ch);
                    current_width += char_width;
                }
                println!("{}", truncated);
            } else {
                println!("{}", line);
            }
        }

        // Navigation information and status display
        self.display_status(start, end)?;

        io::stdout().flush().map_err(|e| {
            AppError::Display(crate::error::DisplayError::TerminalOutput(format!(
                "Failed to flush stdout: {}",
                e
            )))
        })?;

        Ok(())
    }

    /// Display status information
    fn display_status(&self, start: usize, end: usize) -> Result<(), AppError> {
        let total_pages = self.total_items.div_ceil(self.page_size);

        // Move to bottom of screen
        let status_row = self.terminal_height.saturating_sub(2);
        execute!(io::stdout(), cursor::MoveTo(0, status_row)).map_err(|e| {
            AppError::Display(crate::error::DisplayError::TerminalOutput(format!(
                "Failed to move cursor: {}",
                e
            )))
        })?;

        let separator = "─".repeat(self.terminal_width as usize);
        println!("{}", separator);

        let status = if !self.number_input.is_empty() {
            format!(
                "Page number input: {} (Enter to confirm, Esc to cancel)",
                self.number_input
            )
        } else {
            format!(
                "Page {}/{} | Items {}-{}/{} | []/← Page | ↑↓ Scroll | g/G First/Last | ? Help | q Quit",
                self.current_page + 1,
                total_pages,
                start + 1,
                end,
                self.total_items
            )
        };

        println!("{}", status);

        Ok(())
    }

    /// Display help message
    fn show_help(&self) -> Result<(), AppError> {
        execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).map_err(|e| {
            AppError::Display(crate::error::DisplayError::TerminalOutput(format!(
                "Failed to clear screen for help: {}",
                e
            )))
        })?;

        println!("📖 Interactive Pagination Operation Guide");
        println!("═══════════════════════════════════════════════");
        println!();
        println!("📄 Page Navigation:");
        println!("  ]  →  l     Next page");
        println!("  [  ←  h     Previous page");
        println!("  g  Home     First page");
        println!("  G  End      Last page");
        println!("  1-9 Enter   Jump to specific page");
        println!();
        println!("📜 Scrolling:");
        println!("  ↑  k        Scroll up");
        println!("  ↓  j        Scroll down");
        println!("  PageUp      Scroll up 10 lines");
        println!("  PageDown    Scroll down 10 lines");
        println!();
        println!("🔧 Other:");
        println!("  ?           Show this help");
        println!("  q  Esc      Exit");
        println!("  Ctrl+C      Force exit");
        println!();
        println!("💡 Tips:");
        println!("  - Large tables are automatically scrollable");
        println!("  - Enter page number and press Enter to jump directly");
        println!("  - Supports arrow keys and vim-like keys (hjkl)");
        println!();
        println!("Press any key to return...");

        io::stdout().flush().map_err(|e| {
            AppError::Display(crate::error::DisplayError::TerminalOutput(format!(
                "Failed to flush stdout for help: {}",
                e
            )))
        })?;

        // Wait for key input
        event::read().map_err(|e| {
            AppError::Display(crate::error::DisplayError::TerminalOutput(format!(
                "Failed to read key event: {}",
                e
            )))
        })?;

        Ok(())
    }

    /// Calculate maximum scroll offset
    fn calculate_max_scroll<T, F>(&self, items: &[T], renderer: &F) -> usize
    where
        F: Fn(&[T]) -> String,
    {
        let start = self.current_page * self.page_size;
        let end = (start + self.page_size).min(items.len());

        if start >= items.len() {
            return 0;
        }

        let page_items = &items[start..end];
        let table_content = renderer(page_items);
        let line_count = table_content.lines().count();
        let available_height = (self.terminal_height as usize).saturating_sub(3);

        line_count.saturating_sub(available_height)
    }

    /// Move to the next page
    fn next_page(&mut self) {
        let max_page = self.total_items.div_ceil(self.page_size);
        if self.current_page < max_page.saturating_sub(1) {
            self.current_page += 1;
            self.scroll_offset = 0;
        }
    }

    /// Move to the previous page
    fn previous_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
            self.scroll_offset = 0;
        }
    }

    /// Get the current page number
    pub fn current_page(&self) -> usize {
        self.current_page
    }

    /// Get total page count
    pub fn total_pages(&self) -> usize {
        self.total_items.div_ceil(self.page_size)
    }
}
