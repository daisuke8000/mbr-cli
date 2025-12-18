//! Event handling for the TUI application.
//!
//! This module provides keyboard and terminal event handling using crossterm.

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;

/// Terminal events that the application can handle.
#[derive(Debug)]
pub enum Event {
    /// Keyboard input event
    Key(KeyEvent),
    /// Terminal tick for periodic updates
    Tick,
    /// Terminal resize event
    Resize(u16, u16),
}

/// Event handler that polls for terminal events.
pub struct EventHandler {
    /// Tick rate for periodic updates (in milliseconds)
    tick_rate: Duration,
}

impl EventHandler {
    /// Create a new event handler with the specified tick rate.
    pub fn new(tick_rate_ms: u64) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick_rate_ms),
        }
    }

    /// Poll for the next event.
    ///
    /// Returns `Some(Event)` if an event is available, `None` on timeout.
    pub fn next(&self) -> std::io::Result<Event> {
        if event::poll(self.tick_rate)? {
            match event::read()? {
                CrosstermEvent::Key(key) => Ok(Event::Key(key)),
                CrosstermEvent::Resize(w, h) => Ok(Event::Resize(w, h)),
                _ => Ok(Event::Tick),
            }
        } else {
            Ok(Event::Tick)
        }
    }
}
