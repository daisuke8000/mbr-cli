//! mbr-tui - Terminal UI for Metabase
//!
//! A terminal-based interface for interacting with Metabase,
//! similar to lazygit, k9s, or htop.

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::backend::CrosstermBackend;
use std::io::{self, stdout};

mod action;
mod app;
mod components;
mod error;
mod event;
mod layout;
mod service;

use app::App;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    // Set panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));

    // Create and run app (async)
    let mut app = App::new();
    let result = app.run_async(&mut terminal).await;

    // Cleanup terminal
    restore_terminal()?;

    // Report errors
    if let Err(ref err) = result {
        eprintln!("Application error: {:?}", err);
    }

    result
}

/// Restore terminal to normal state.
fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
