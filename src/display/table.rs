use crate::api::models::{Dashboard, DashboardCard, QueryResult, Question};
use crate::error::AppError;
use comfy_table::{Attribute, Cell, Color, Table, presets};
use crossterm::terminal;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Parameters for question header display
pub struct QuestionHeaderParams<'a> {
    pub question_id: u32,
    pub question_name: &'a str,
    pub total_records: usize,
    pub current_page: Option<usize>,
    pub total_pages: Option<usize>,
    pub start_row: Option<usize>,
    pub end_row: Option<usize>,
}

/// Parameters for table header information display
#[derive(Debug, Clone)]
pub struct TableHeaderInfo {
    /// Data source information (question name, API endpoint, etc.)
    pub data_source: String,
    /// Data source ID (question ID, etc.)
    pub source_id: Option<u32>,
    /// Total record count
    pub total_records: usize,
    /// Display start position (1-based)
    pub start_position: usize,
    /// Display end position (1-based)
    pub end_position: usize,
    /// Offset information
    pub offset: Option<usize>,
    /// Filter application status
    pub is_filtered: bool,
    /// Pagination information
    pub pagination_info: Option<PaginationInfo>,
}

/// Pagination display information
#[derive(Debug, Clone)]
pub struct PaginationInfo {
    pub current_page: usize,
    pub total_pages: usize,
    pub page_size: usize,
}

/// Formatter and utilities for table display
pub struct TableDisplay {
    max_width: Option<usize>,
    use_colors: bool,
}

impl TableDisplay {
    /// Create a new TableDisplay instance
    pub fn new() -> Self {
        Self {
            max_width: Self::detect_terminal_width(),
            use_colors: true,
        }
    }

    /// Detect terminal width
    fn detect_terminal_width() -> Option<usize> {
        match terminal::size() {
            Ok((cols, _rows)) => {
                let width = cols as usize;
                // Set minimum and maximum width for improved stability
                if width < 40 {
                    Some(40) // Minimum width
                } else if width > 200 {
                    Some(200) // Maximum width
                } else {
                    Some(width)
                }
            }
            Err(_) => Some(80), // Default width
        }
    }

    /// Create a TableDisplay instance with maximum width setting
    pub fn with_max_width(mut self, width: usize) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Set color usage
    pub fn with_colors(mut self, use_colors: bool) -> Self {
        self.use_colors = use_colors;
        self
    }

    /// Render a question list in table format
    pub fn render_question_list(&self, questions: &[Question]) -> Result<String, AppError> {
        self.render_question_list_with_limit(questions, None)
    }

    /// Render a question list in table format with a limit
    pub fn render_question_list_with_limit(
        &self,
        questions: &[Question],
        limit: Option<usize>,
    ) -> Result<String, AppError> {
        let mut table = Table::new();

        // UTF8 style and header configuration
        table.load_preset(presets::UTF8_FULL);
        table.set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

        // Table width setting adjusted to terminal width
        self.configure_table_width(&mut table);

        if self.use_colors {
            table.set_header(vec![
                Cell::new("ID")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Cyan),
                Cell::new("Name")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Cyan),
                Cell::new("Collection")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Cyan),
                Cell::new("Description")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Cyan),
            ]);
        } else {
            table.set_header(vec!["ID", "Name", "Collection", "Description"]);
        }

        // Add data rows (with limit applied)
        let display_questions = if let Some(limit_val) = limit {
            &questions[..questions.len().min(limit_val)]
        } else {
            questions
        };

        // Get responsive column widths
        let (_id_width, name_width, collection_width, desc_width) =
            self.get_responsive_column_widths();

        for question in display_questions {
            let collection_name = self.extract_collection_name(question);
            let description = question.description.as_deref().unwrap_or("N/A");

            let row = vec![
                if self.use_colors {
                    Cell::new(question.id.to_string()).fg(Color::Cyan)
                } else {
                    Cell::new(question.id.to_string())
                },
                Cell::new(self.truncate_text(&question.name, name_width)),
                Cell::new(self.truncate_text(&collection_name, collection_width)),
                if self.use_colors {
                    Cell::new(self.truncate_text(description, desc_width)).fg(Color::DarkGrey)
                } else {
                    Cell::new(self.truncate_text(description, desc_width))
                },
            ];

            table.add_row(row);
        }

        // Omission display due to limit
        if let Some(limit_val) = limit {
            if questions.len() > limit_val {
                let remaining = questions.len() - limit_val;
                let note = format!(
                    "... and {} more questions (use --full to see all)",
                    remaining
                );

                if self.use_colors {
                    table.add_row(vec![
                        Cell::new(""),
                        Cell::new(note)
                            .fg(Color::DarkGrey)
                            .add_attribute(Attribute::Italic),
                        Cell::new(""),
                        Cell::new(""),
                    ]);
                } else {
                    table.add_row(vec!["", &note, "", ""]);
                }
            }
        }

        Ok(table.to_string())
    }

    /// Render query result in table format (based on the original implementation, wrap support)
    pub fn render_query_result(&self, result: &QueryResult) -> Result<String, AppError> {
        self.render_query_result_with_limit(result, None)
    }

    /// Render query result in table format with limit (based on original implementation, wrap support)
    pub fn render_query_result_with_limit(
        &self,
        result: &QueryResult,
        limit: Option<usize>,
    ) -> Result<String, AppError> {
        if result.data.rows.is_empty() {
            return Ok("Query returned no results.".to_string());
        }

        let total_rows = result.data.rows.len();
        let display_limit = limit.unwrap_or(total_rows);
        let rows_to_display = display_limit.min(total_rows);

        let mut table = Table::new();
        // Same Dynamic setting as the original implementation-enable wrap display
        table.set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

        // Header configuration (based on the original implementation)
        let headers: Vec<Cell> = result
            .data
            .cols
            .iter()
            .map(|col| {
                if self.use_colors {
                    Cell::new(&col.display_name)
                        .add_attribute(Attribute::Bold)
                        .fg(Color::Green)
                } else {
                    Cell::new(&col.display_name).add_attribute(Attribute::Bold)
                }
            })
            .collect();
        table.set_header(headers);

        // Add data rows (based on the original implementation, with limit applied)
        for row in result.data.rows.iter().take(rows_to_display) {
            let cells: Vec<Cell> = row
                .iter()
                .map(|value| {
                    let formatted_value = self.format_cell_value(value);
                    if self.use_colors && matches!(value, serde_json::Value::Null) {
                        Cell::new(formatted_value)
                            .fg(Color::DarkGrey)
                            .add_attribute(Attribute::Italic)
                    } else {
                        Cell::new(formatted_value)
                    }
                })
                .collect();
            table.add_row(cells);
        }

        let mut output = table.to_string();

        // Display omission information due to limit
        if rows_to_display != total_rows {
            output.push_str(&format!(
                "\nShowing {} of {} rows",
                rows_to_display, total_rows
            ));
        }

        Ok(output)
    }

    /// Extended display including question header and result information
    pub fn render_question_header_with_results(&self, params: &QuestionHeaderParams) -> String {
        let mut header = String::new();

        // Question information
        header.push_str(&format!(
            "🚀 Question #{}: {}\n",
            params.question_id, params.question_name
        ));

        // Result information
        if let (Some(current), Some(total), Some(start), Some(end)) = (
            params.current_page,
            params.total_pages,
            params.start_row,
            params.end_row,
        ) {
            header.push_str(&format!(
                "📊 Question execution result: {} total | Page {}/{} ({}-{} / {} records)\n",
                params.total_records,
                current + 1,
                total,
                start + 1,
                end,
                params.total_records
            ));
        } else {
            header.push_str(&format!(
                "📊 Question execution result: {} total records\n",
                params.total_records
            ));
        }

        // Execution time (current time)
        let now = std::time::SystemTime::now();
        header.push_str(&format!(
            "⏰ Execution time: {:?} | 💡 Tips: --format json/csv\n",
            now
        ));

        header.push_str("──────────────────────────────────────────────────────────\n");

        header
    }

    /// Display comprehensive table header information
    ///
    /// Following the PaginationInfo concept from reference implementation, generates comprehensive header including
    /// - Data source information (question name, ID, etc.)
    /// - Record count and pagination status
    /// - Offset information
    /// - Filter application status
    pub fn render_comprehensive_header(&self, info: &TableHeaderInfo) -> String {
        let mut header = String::new();

        // Data source information
        if let Some(id) = info.source_id {
            header.push_str(&format!(
                "🚀 {}: {} (ID: {})\n",
                if info.data_source.contains("Question") {
                    "Question"
                } else {
                    "Data"
                },
                info.data_source,
                id
            ));
        } else {
            header.push_str(&format!("🚀 Data: {}\n", info.data_source));
        }

        // Data range information
        let range_info = if info.start_position == info.end_position {
            format!("Display: {} record", info.start_position)
        } else {
            format!(
                "Display: records {}-{}",
                info.start_position, info.end_position
            )
        };

        let total_info = format!("Total records: {}", info.total_records);

        header.push_str(&format!("📊 {} | {}", range_info, total_info));

        // Offset information
        if let Some(offset) = info.offset {
            if offset > 0 {
                header.push_str(&format!(" | Offset: +{}", offset));
            }
        }

        // Filter application status
        if info.is_filtered {
            header.push_str(" | 🔍 Filter applied");
        }

        header.push('\n');

        // Pagination information
        if let Some(ref page_info) = info.pagination_info {
            header.push_str(&format!(
                "📄 Page: {}/{} | Page size: {} records\n",
                page_info.current_page + 1,
                page_info.total_pages,
                page_info.page_size
            ));
        }

        // Execution time and tips
        let now = std::time::SystemTime::now();
        header.push_str(&format!(
            "⏰ Execution time: {:?} | 💡 Tips: --limit, --offset, --format\n",
            now
        ));

        // Separator
        let terminal_width = self.max_width.unwrap_or(80);
        let separator = "─".repeat(terminal_width.min(80));
        header.push_str(&format!("{}\n", separator));

        header
    }

    /// Builder helper for TableHeaderInfo
    pub fn create_header_info_builder() -> TableHeaderInfoBuilder {
        TableHeaderInfoBuilder::new()
    }

    /// Extract collection name from question
    fn extract_collection_name(&self, question: &Question) -> String {
        if let Some(ref collection) = question.collection {
            collection.name.clone()
        } else if question.collection_id.is_some() {
            format!("ID: {}", question.collection_id.unwrap())
        } else {
            "Root".to_string()
        }
    }

    /// Set table width to match the terminal size
    fn configure_table_width(&self, table: &mut Table) {
        if let Some(terminal_width) = self.max_width {
            // Adjust considering borders and padding from terminal width
            let available_width = if terminal_width > 20 {
                terminal_width - 6 // Consider left/right borders, padding, margins
            } else {
                terminal_width.max(40) // Ensure minimum width
            };

            table.set_width(available_width as u16);
        } else {
            // Default width when terminal size cannot be obtained
            table.set_width(80);
        }
    }

    /// Calculate responsive column widths (for question list)
    fn get_responsive_column_widths(&self) -> (usize, usize, usize, usize) {
        let terminal_width = self.max_width.unwrap_or(80);

        if terminal_width < 60 {
            // Very narrow terminal
            (3, 10, 6, 15)
        } else if terminal_width < 80 {
            // Narrow terminal
            (4, 15, 8, 20)
        } else if terminal_width < 120 {
            // Standard terminal
            (4, 25, 12, 25)
        } else {
            // Wide terminal
            (4, 40, 20, 35)
        }
    }

    /// Truncate text to specified width and add ellipsis
    fn truncate_text(&self, text: &str, max_width: usize) -> String {
        if text.width() <= max_width {
            return text.to_string();
        }

        let ellipsis = "...";
        let ellipsis_width = ellipsis.width();

        if max_width <= ellipsis_width {
            return ellipsis[..max_width].to_string();
        }

        let target_width = max_width - ellipsis_width;
        let mut result = String::new();
        let mut current_width = 0;

        for ch in text.chars() {
            let ch_width = ch.width().unwrap_or(0);
            if current_width + ch_width > target_width {
                break;
            }
            result.push(ch);
            current_width += ch_width;
        }

        result.push_str(ellipsis);
        result
    }

    /// Format cell value
    pub fn format_cell_value(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Null => {
                "-".to_string() // Shorter and more stable display
            }
            serde_json::Value::String(s) => {
                // Long strings are automatically truncated
                if s.len() > 100 {
                    self.truncate_text(s, 100)
                } else {
                    s.clone()
                }
            }
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Array(arr) => {
                if arr.is_empty() {
                    "[]".to_string()
                } else {
                    format!("[{} items]", arr.len())
                }
            }
            serde_json::Value::Object(obj) => {
                if obj.is_empty() {
                    "{}".to_string()
                } else {
                    format!("{{{} items}}", obj.len())
                }
            }
        }
    }

    /// Pad text to specified width
    pub fn pad_to_width(&self, text: &str, width: usize) -> String {
        let text_width = text.width();
        if text_width >= width {
            text.to_string()
        } else {
            format!("{}{}", text, " ".repeat(width - text_width))
        }
    }

    /// Center-align text
    pub fn center_text(&self, text: &str, width: usize) -> String {
        let text_width = text.width();
        if text_width >= width {
            return text.to_string();
        }

        let padding = width - text_width;
        let left_padding = padding / 2;
        let right_padding = padding - left_padding;

        format!(
            "{}{}{}",
            " ".repeat(left_padding),
            text,
            " ".repeat(right_padding)
        )
    }
}

impl Default for TableDisplay {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for TableHeaderInfo
#[derive(Debug)]
pub struct TableHeaderInfoBuilder {
    data_source: Option<String>,
    source_id: Option<u32>,
    total_records: Option<usize>,
    start_position: Option<usize>,
    end_position: Option<usize>,
    offset: Option<usize>,
    is_filtered: bool,
    pagination_info: Option<PaginationInfo>,
}

impl TableHeaderInfoBuilder {
    pub fn new() -> Self {
        Self {
            data_source: None,
            source_id: None,
            total_records: None,
            start_position: None,
            end_position: None,
            offset: None,
            is_filtered: false,
            pagination_info: None,
        }
    }

    pub fn data_source(mut self, source: String) -> Self {
        self.data_source = Some(source);
        self
    }

    pub fn source_id(mut self, id: u32) -> Self {
        self.source_id = Some(id);
        self
    }

    pub fn total_records(mut self, total: usize) -> Self {
        self.total_records = Some(total);
        self
    }

    pub fn display_range(mut self, start: usize, end: usize) -> Self {
        self.start_position = Some(start);
        self.end_position = Some(end);
        self
    }

    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn filtered(mut self) -> Self {
        self.is_filtered = true;
        self
    }

    pub fn pagination(mut self, current_page: usize, total_pages: usize, page_size: usize) -> Self {
        self.pagination_info = Some(PaginationInfo {
            current_page,
            total_pages,
            page_size,
        });
        self
    }

    pub fn build(self) -> Result<TableHeaderInfo, AppError> {
        Ok(TableHeaderInfo {
            data_source: self.data_source.unwrap_or_else(|| "Unknown".to_string()),
            source_id: self.source_id,
            total_records: self.total_records.unwrap_or(0),
            start_position: self.start_position.unwrap_or(1),
            end_position: self.end_position.unwrap_or(self.total_records.unwrap_or(0)),
            offset: self.offset,
            is_filtered: self.is_filtered,
            pagination_info: self.pagination_info,
        })
    }
}

impl Default for TableHeaderInfoBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TableDisplay {
    /// Render dashboard list in table format with dynamic field-based headers
    pub fn render_dashboard_list(&self, dashboards: &[Dashboard]) -> Result<String, AppError> {
        if dashboards.is_empty() {
            return Ok("No dashboards found.".to_string());
        }

        let mut table = Table::new();
        table.set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

        // Dynamic headers based on Dashboard struct fields
        let headers = [
            "ID",
            "Name",
            "Description",
            "Collection ID",
            "Creator ID",
            "Created At",
            "Updated At",
        ];

        if self.use_colors {
            let colored_headers: Vec<Cell> = headers
                .iter()
                .map(|h| Cell::new(h).add_attribute(Attribute::Bold).fg(Color::Green))
                .collect();
            table.set_header(colored_headers);
        } else {
            let bold_headers: Vec<Cell> = headers
                .iter()
                .map(|h| Cell::new(h).add_attribute(Attribute::Bold))
                .collect();
            table.set_header(bold_headers);
        }

        // Add data rows with field values
        for dashboard in dashboards {
            let description = dashboard.description.as_deref().unwrap_or("N/A");
            let collection_id = dashboard
                .collection_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "Root".to_string());
            let creator_id = dashboard
                .creator_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "N/A".to_string());

            let row = vec![
                if self.use_colors {
                    Cell::new(dashboard.id.to_string()).fg(Color::Cyan)
                } else {
                    Cell::new(dashboard.id.to_string())
                },
                Cell::new(&dashboard.name),
                Cell::new(description),
                Cell::new(collection_id),
                Cell::new(creator_id),
                Cell::new(self.format_datetime(&dashboard.created_at)),
                Cell::new(self.format_datetime(&dashboard.updated_at)),
            ];

            table.add_row(row);
        }

        Ok(table.to_string())
    }

    /// Render dashboard details in table format with field-based structure
    pub fn render_dashboard_details(&self, dashboard: &Dashboard) -> Result<String, AppError> {
        let mut table = Table::new();
        table.set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

        // Two-column layout: Field Name | Value
        if self.use_colors {
            table.set_header(vec![
                Cell::new("Field")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Green),
                Cell::new("Value")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Green),
            ]);
        } else {
            table.set_header(vec![
                Cell::new("Field").add_attribute(Attribute::Bold),
                Cell::new("Value").add_attribute(Attribute::Bold),
            ]);
        }

        // Add each field as a row based on Dashboard struct
        let fields = vec![
            ("ID", dashboard.id.to_string()),
            ("Name", dashboard.name.clone()),
            (
                "Description",
                dashboard
                    .description
                    .clone()
                    .unwrap_or_else(|| "N/A".to_string()),
            ),
            (
                "Collection ID",
                dashboard
                    .collection_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "Root".to_string()),
            ),
            (
                "Creator ID",
                dashboard
                    .creator_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "N/A".to_string()),
            ),
            ("Created At", dashboard.created_at.clone()),
            ("Updated At", dashboard.updated_at.clone()),
            (
                "Cards Count",
                dashboard
                    .dashcards
                    .as_ref()
                    .map(|c| c.len().to_string())
                    .unwrap_or_else(|| "0".to_string()),
            ),
        ];

        for (field_name, field_value) in fields {
            let row = vec![
                if self.use_colors {
                    Cell::new(field_name).fg(Color::Yellow)
                } else {
                    Cell::new(field_name)
                },
                Cell::new(field_value),
            ];
            table.add_row(row);
        }

        Ok(table.to_string())
    }

    /// Render dashboard cards in table format with field-based headers
    pub fn render_dashboard_cards(&self, cards: &[DashboardCard]) -> Result<String, AppError> {
        if cards.is_empty() {
            return Ok("No cards found for this dashboard.".to_string());
        }

        let mut table = Table::new();
        table.set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

        // Dynamic headers based on DashboardCard struct fields
        let headers = [
            "ID",
            "Dashboard ID",
            "Card ID",
            "Col",
            "Row",
            "Size X",
            "Size Y",
        ];

        if self.use_colors {
            let colored_headers: Vec<Cell> = headers
                .iter()
                .map(|h| Cell::new(h).add_attribute(Attribute::Bold).fg(Color::Green))
                .collect();
            table.set_header(colored_headers);
        } else {
            let bold_headers: Vec<Cell> = headers
                .iter()
                .map(|h| Cell::new(h).add_attribute(Attribute::Bold))
                .collect();
            table.set_header(bold_headers);
        }

        // Add data rows with field values from DashboardCard struct
        for card in cards {
            let card_id = card
                .card_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "NULL".to_string());

            let row = vec![
                if self.use_colors {
                    Cell::new(card.id.to_string()).fg(Color::Cyan)
                } else {
                    Cell::new(card.id.to_string())
                },
                Cell::new(card.dashboard_id.to_string()),
                Cell::new(card_id),
                Cell::new(card.col.to_string()),
                Cell::new(card.row.to_string()),
                Cell::new(card.size_x.to_string()),
                Cell::new(card.size_y.to_string()),
            ];

            table.add_row(row);
        }

        Ok(table.to_string())
    }

    /// Helper to format datetime for dashboard display
    fn format_datetime(&self, datetime: &str) -> String {
        // Simple date formatting - extract date part from ISO datetime
        if let Some(date_part) = datetime.split('T').next() {
            date_part.to_string()
        } else {
            datetime.chars().take(16).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::{Collection, Column, QueryData, QueryResult, Question};
    use serde_json::json;

    fn create_test_question(id: u32, name: &str) -> Question {
        Question {
            id,
            name: name.to_string(),
            description: Some("Test description".to_string()),
            collection_id: Some(1),
            collection: Some(Collection {
                id: Some(1),
                name: "Test Collection".to_string(),
            }),
        }
    }

    fn create_test_query_result() -> QueryResult {
        QueryResult {
            data: QueryData {
                cols: vec![
                    Column {
                        name: "id".to_string(),
                        display_name: "ID".to_string(),
                        base_type: "type/Integer".to_string(),
                    },
                    Column {
                        name: "name".to_string(),
                        display_name: "Name".to_string(),
                        base_type: "type/Text".to_string(),
                    },
                ],
                rows: vec![
                    vec![json!(1), json!("Alice")],
                    vec![json!(2), json!("Bob")],
                    vec![json!(3), json!(null)],
                ],
            },
        }
    }

    #[test]
    fn test_table_display_creation() {
        let display = TableDisplay::new();
        // In test environment, terminal size may be detected, so only check for value existence
        assert!(display.use_colors);

        let display = TableDisplay::new().with_max_width(80).with_colors(false);
        assert_eq!(display.max_width, Some(80));
        assert!(!display.use_colors);
    }

    #[test]
    fn test_truncate_text() {
        let display = TableDisplay::new();

        // Short text remains unchanged
        assert_eq!(display.truncate_text("Hello", 10), "Hello");

        // Long text is truncated
        assert_eq!(display.truncate_text("Hello World", 8), "Hello...");

        // Unicode character processing (Japanese characters = 2 width per character)
        // "Hello World Example" = 14 width, target 8 width so "Hello..." = 7 width
        assert_eq!(display.truncate_text("Hello World!", 8), "Hello...");
    }

    #[test]
    fn test_pad_to_width() {
        let display = TableDisplay::new();

        assert_eq!(display.pad_to_width("Hello", 10), "Hello     ");
        assert_eq!(display.pad_to_width("Hello World", 5), "Hello World");

        // "Wide Text" = 9 width, so no change for width 8
        assert_eq!(display.pad_to_width("Wide Text", 8), "Wide Text");
        // "Text" = 4 width, so add 6 spaces for width 10
        assert_eq!(display.pad_to_width("Text", 10), "Text      ");
    }

    #[test]
    fn test_center_text() {
        let display = TableDisplay::new();

        assert_eq!(display.center_text("Hi", 6), "  Hi  ");
        assert_eq!(display.center_text("Hello", 5), "Hello");

        // "Long Text" = 9 width, so no change for width 8
        assert_eq!(display.center_text("Long Text", 8), "Long Text");
        // "Hi" = 2 width, 5+5 padding for width 12
        assert_eq!(display.center_text("Hi", 12), "     Hi     ");
    }

    #[test]
    fn test_format_cell_value() {
        let display = TableDisplay::new();

        assert_eq!(display.format_cell_value(&json!(null)), "-");
        assert_eq!(display.format_cell_value(&json!("text")), "text");
        assert_eq!(display.format_cell_value(&json!(123)), "123");
        assert_eq!(display.format_cell_value(&json!(true)), "true");
    }

    #[test]
    fn test_render_question_list() {
        let display = TableDisplay::new();
        let questions = vec![
            create_test_question(1, "Test Question 1"),
            create_test_question(2, "Test Question 2"),
        ];

        let result = display.render_question_list(&questions);
        assert!(result.is_ok());

        let table_str = result.unwrap();
        assert!(table_str.contains("Test Question 1"));
        assert!(table_str.contains("Test Question 2"));
        // Collection names are truncated, so check with partial match
        assert!(table_str.contains("Test Coll"));
    }

    #[test]
    fn test_render_query_result() {
        let display = TableDisplay::new();
        let result = create_test_query_result();

        let rendered = display.render_query_result(&result);
        assert!(rendered.is_ok());

        let table_str = rendered.unwrap();
        assert!(table_str.contains("ID"));
        assert!(table_str.contains("Name"));
        assert!(table_str.contains("Alice"));
        assert!(table_str.contains("Bob"));
        assert!(table_str.contains("-"));
    }

    #[test]
    fn test_extract_collection_name() {
        let display = TableDisplay::new();

        // With Collection object
        let question_with_collection = create_test_question(1, "Test");
        assert_eq!(
            display.extract_collection_name(&question_with_collection),
            "Test Collection"
        );

        // collection_id only
        let question_with_id = Question {
            id: 2,
            name: "Test".to_string(),
            description: None,
            collection_id: Some(42),
            collection: None,
        };
        assert_eq!(display.extract_collection_name(&question_with_id), "ID: 42");

        // Both are None
        let question_root = Question {
            id: 3,
            name: "Test".to_string(),
            description: None,
            collection_id: None,
            collection: None,
        };
        assert_eq!(display.extract_collection_name(&question_root), "Root");
    }
}
