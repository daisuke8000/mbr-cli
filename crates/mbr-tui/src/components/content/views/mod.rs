//! View rendering functions for ContentPanel.
//!
//! This module contains all the render_* functions that draw different
//! views (Welcome, Questions, Collections, Databases, QueryResult, etc.).
//!
//! ## Module Structure
//! - `welcome.rs`: Welcome screen and placeholder rendering
//! - `lists.rs`: Questions, Collections, Databases list views
//! - `drill_down.rs`: Collection questions, schemas, tables drill-down views
//! - `results.rs`: Query result and table preview rendering

mod drill_down;
mod lists;
mod results;
mod welcome;
