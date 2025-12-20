use crate::api::models::{QueryResult, Question};
use crate::error::AppError;
use crate::utils::text::truncate_text;
use comfy_table::{Attribute, Cell, Color, Table, presets};
use crossterm::terminal;

const HEADER_CAPACITY: usize = 256;
const COMPREHENSIVE_HEADER_CAPACITY: usize = 512;

pub struct QuestionHeaderParams<'a> {
    pub question_id: u32,
    pub question_name: &'a str,
    pub total_records: usize,
    pub current_page: Option<usize>,
    pub total_pages: Option<usize>,
    pub start_row: Option<usize>,
    pub end_row: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct TableHeaderInfo {
    pub data_source: String,
    pub source_id: Option<u32>,
    pub total_records: usize,
    pub start_position: usize,
    pub end_position: usize,
    pub offset: Option<usize>,
    pub is_filtered: bool,
    pub pagination_info: Option<PaginationInfo>,
}

#[derive(Debug, Clone)]
pub struct PaginationInfo {
    pub current_page: usize,
    pub total_pages: usize,
    pub page_size: usize,
}

struct ColumnWidths {
    name: usize,
    collection: usize,
    description: usize,
}

pub struct TableDisplay {
    max_width: Option<usize>,
    use_colors: bool,
}

impl TableDisplay {
    pub fn new() -> Self {
        Self {
            max_width: Self::detect_terminal_width(),
            use_colors: true,
        }
    }

    fn detect_terminal_width() -> Option<usize> {
        match terminal::size() {
            Ok((cols, _)) => {
                let width = cols as usize;
                Some(width.clamp(40, 200))
            }
            Err(_) => Some(80),
        }
    }

    pub fn with_max_width(mut self, width: usize) -> Self {
        self.max_width = Some(width);
        self
    }

    pub fn with_colors(mut self, use_colors: bool) -> Self {
        self.use_colors = use_colors;
        self
    }

    fn bold_header(&self, text: &str, color: Color) -> Cell {
        if self.use_colors {
            Cell::new(text).add_attribute(Attribute::Bold).fg(color)
        } else {
            Cell::new(text).add_attribute(Attribute::Bold)
        }
    }

    fn colored_cell(&self, text: &str, color: Color) -> Cell {
        if self.use_colors {
            Cell::new(text).fg(color)
        } else {
            Cell::new(text)
        }
    }

    fn set_colored_headers(&self, table: &mut Table, headers: &[&str], color: Color) {
        let cells: Vec<Cell> = headers.iter().map(|h| self.bold_header(h, color)).collect();
        table.set_header(cells);
    }

    pub fn render_question_list(&self, questions: &[Question]) -> Result<String, AppError> {
        self.render_question_list_with_limit(questions, None)
    }

    pub fn render_question_list_with_limit(
        &self,
        questions: &[Question],
        limit: Option<usize>,
    ) -> Result<String, AppError> {
        let mut table = Table::new();
        table.load_preset(presets::UTF8_FULL);
        table.set_content_arrangement(comfy_table::ContentArrangement::Dynamic);
        self.configure_table_width(&mut table);
        self.set_colored_headers(
            &mut table,
            &["ID", "Name", "Collection", "Description"],
            Color::Cyan,
        );

        let display_questions = match limit {
            Some(l) => &questions[..questions.len().min(l)],
            None => questions,
        };

        let widths = self.get_responsive_column_widths();

        for question in display_questions {
            let collection_name = self.extract_collection_name(question);
            let description = question.description.as_deref().unwrap_or("N/A");

            table.add_row(vec![
                self.colored_cell(&question.id.to_string(), Color::Cyan),
                Cell::new(truncate_text(&question.name, widths.name)),
                Cell::new(truncate_text(&collection_name, widths.collection)),
                self.colored_cell(
                    &truncate_text(description, widths.description),
                    Color::DarkGrey,
                ),
            ]);
        }

        if let Some(limit_val) = limit {
            if questions.len() > limit_val {
                let note = format!(
                    "... and {} more questions (use --full to see all)",
                    questions.len() - limit_val
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

    pub fn render_query_result(&self, result: &QueryResult) -> Result<String, AppError> {
        self.render_query_result_with_limit(result, None)
    }

    pub fn render_query_result_with_limit(
        &self,
        result: &QueryResult,
        limit: Option<usize>,
    ) -> Result<String, AppError> {
        if result.data.rows.is_empty() {
            return Ok("Query returned no results.".to_string());
        }

        let total_rows = result.data.rows.len();
        let rows_to_display = limit.unwrap_or(total_rows).min(total_rows);

        let mut table = Table::new();
        table.set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

        let headers: Vec<Cell> = result
            .data
            .cols
            .iter()
            .map(|col| self.bold_header(&col.display_name, Color::Green))
            .collect();
        table.set_header(headers);

        for row in result.data.rows.iter().take(rows_to_display) {
            let cells: Vec<Cell> = row
                .iter()
                .map(|value| {
                    let formatted = self.format_cell_value(value);
                    if self.use_colors && matches!(value, serde_json::Value::Null) {
                        Cell::new(formatted)
                            .fg(Color::DarkGrey)
                            .add_attribute(Attribute::Italic)
                    } else {
                        Cell::new(formatted)
                    }
                })
                .collect();
            table.add_row(cells);
        }

        let mut output = table.to_string();
        if rows_to_display != total_rows {
            output.push_str(&format!(
                "\nShowing {} of {} rows",
                rows_to_display, total_rows
            ));
        }

        Ok(output)
    }

    pub fn render_question_header_with_results(&self, params: &QuestionHeaderParams) -> String {
        let mut header = String::with_capacity(HEADER_CAPACITY);

        header.push_str(&format!(
            "🚀 Question #{}: {}\n",
            params.question_id, params.question_name
        ));

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

        header.push_str(&format!(
            "⏰ Execution time: {:?} | 💡 Tips: --format json/csv\n",
            std::time::SystemTime::now()
        ));
        header.push_str("──────────────────────────────────────────────────────────\n");

        header
    }

    pub fn render_comprehensive_header(&self, info: &TableHeaderInfo) -> String {
        let mut header = String::with_capacity(COMPREHENSIVE_HEADER_CAPACITY);

        if let Some(id) = info.source_id {
            let label = if info.data_source.contains("Question") {
                "Question"
            } else {
                "Data"
            };
            header.push_str(&format!(
                "🚀 {}: {} (ID: {})\n",
                label, info.data_source, id
            ));
        } else {
            header.push_str(&format!("🚀 Data: {}\n", info.data_source));
        }

        let range_info = if info.start_position == info.end_position {
            format!("Display: {} record", info.start_position)
        } else {
            format!(
                "Display: records {}-{}",
                info.start_position, info.end_position
            )
        };

        header.push_str(&format!(
            "📊 {} | Total records: {}",
            range_info, info.total_records
        ));

        if let Some(offset) = info.offset {
            if offset > 0 {
                header.push_str(&format!(" | Offset: +{}", offset));
            }
        }

        if info.is_filtered {
            header.push_str(" | 🔍 Filter applied");
        }

        header.push('\n');

        if let Some(ref page_info) = info.pagination_info {
            header.push_str(&format!(
                "📄 Page: {}/{} | Page size: {} records\n",
                page_info.current_page + 1,
                page_info.total_pages,
                page_info.page_size
            ));
        }

        header.push_str(&format!(
            "⏰ Execution time: {:?} | 💡 Tips: --limit, --offset, --format\n",
            std::time::SystemTime::now()
        ));
        header.push_str(&format!(
            "{}\n",
            "─".repeat(self.max_width.unwrap_or(80).min(80))
        ));

        header
    }

    pub fn create_header_info_builder() -> TableHeaderInfoBuilder {
        TableHeaderInfoBuilder::new()
    }

    fn extract_collection_name(&self, question: &Question) -> String {
        if let Some(ref collection) = question.collection {
            collection.name.clone()
        } else if let Some(collection_id) = question.collection_id {
            format!("ID: {}", collection_id)
        } else {
            "Root".to_string()
        }
    }

    fn configure_table_width(&self, table: &mut Table) {
        let width = self
            .max_width
            .map(|w| if w > 20 { w - 6 } else { w.max(40) })
            .unwrap_or(80);
        table.set_width(width as u16);
    }

    fn get_responsive_column_widths(&self) -> ColumnWidths {
        match self.max_width.unwrap_or(80) {
            0..=59 => ColumnWidths {
                name: 10,
                collection: 6,
                description: 15,
            },
            60..=79 => ColumnWidths {
                name: 15,
                collection: 8,
                description: 20,
            },
            80..=119 => ColumnWidths {
                name: 25,
                collection: 12,
                description: 25,
            },
            _ => ColumnWidths {
                name: 40,
                collection: 20,
                description: 35,
            },
        }
    }

    pub fn format_cell_value(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Null => "-".to_string(),
            serde_json::Value::String(s) if s.len() > 100 => truncate_text(s, 100),
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Array(arr) if arr.is_empty() => "[]".to_string(),
            serde_json::Value::Array(arr) => format!("[{} items]", arr.len()),
            serde_json::Value::Object(obj) if obj.is_empty() => "{}".to_string(),
            serde_json::Value::Object(obj) => format!("{{{} items}}", obj.len()),
        }
    }

    /// Render a simple table with custom headers and rows
    pub fn render_simple_table(&self, headers: &[&str], rows: &[Vec<String>]) -> String {
        let mut table = Table::new();
        table.load_preset(presets::UTF8_FULL);
        table.set_content_arrangement(comfy_table::ContentArrangement::Dynamic);
        self.configure_table_width(&mut table);
        self.set_colored_headers(&mut table, headers, Color::Cyan);

        for row in rows {
            let cells: Vec<Cell> = row.iter().map(Cell::new).collect();
            table.add_row(cells);
        }

        table.to_string()
    }
}

impl Default for TableDisplay {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default)]
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
        Self::default()
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

    pub fn build(self) -> TableHeaderInfo {
        let total_records = self.total_records.unwrap_or(0);
        TableHeaderInfo {
            data_source: self.data_source.unwrap_or_else(|| "Unknown".to_string()),
            source_id: self.source_id,
            total_records,
            start_position: self.start_position.unwrap_or(1),
            end_position: self.end_position.unwrap_or(total_records),
            offset: self.offset,
            is_filtered: self.is_filtered,
            pagination_info: self.pagination_info,
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
    fn test_format_cell_value() {
        let display = TableDisplay::new();

        // Basic types
        assert_eq!(display.format_cell_value(&json!(null)), "-");
        assert_eq!(display.format_cell_value(&json!("text")), "text");
        assert_eq!(display.format_cell_value(&json!(123)), "123");
        assert_eq!(display.format_cell_value(&json!(true)), "true");

        // Long string truncation (> 100 chars)
        let long_string = "a".repeat(150);
        let result = display.format_cell_value(&json!(long_string));
        assert!(result.len() <= 103); // 100 + "..."
        assert!(result.ends_with("..."));

        // Empty collections
        assert_eq!(display.format_cell_value(&json!([])), "[]");
        assert_eq!(display.format_cell_value(&json!({})), "{}");

        // Non-empty collections
        assert_eq!(display.format_cell_value(&json!([1, 2, 3])), "[3 items]");
        assert_eq!(
            display.format_cell_value(&json!({"a": 1, "b": 2})),
            "{2 items}"
        );
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

        let table_str = result.expect("Failed to render question list");
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

        let table_str = rendered.expect("Failed to render query result");
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
