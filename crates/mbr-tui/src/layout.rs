//! Layout constants for mbr-tui.
//!
//! Centralizes all layout-related magic numbers for easy tuning and consistency.

/// Main layout constants.
pub mod main {
    /// Header panel height in rows (includes tab bar).
    pub const HEADER_HEIGHT: u16 = 3;

    /// Status bar height in rows.
    pub const STATUS_BAR_HEIGHT: u16 = 3;
}

/// Questions table column widths.
pub mod questions_table {
    /// ID column width.
    pub const ID_WIDTH: u16 = 6;

    /// Name column minimum width.
    pub const NAME_MIN_WIDTH: u16 = 20;

    /// Collection column width.
    pub const COLLECTION_WIDTH: u16 = 20;
}
