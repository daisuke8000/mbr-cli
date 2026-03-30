//! Event handling for the TUI application.
//!
//! This module previously provided a polling-based EventHandler.
//! Now the event loop uses crossterm's async EventStream via tokio::select!
//! for event-driven rendering (no fixed tick polling).
//!
//! This module is retained for backward compatibility of the module declaration.
