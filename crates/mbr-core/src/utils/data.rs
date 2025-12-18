use crate::api::models::{QueryData, QueryResult};
use crate::error::AppError;

/// Manages offset for --offset option pagination
#[derive(Debug, Clone)]
pub struct OffsetManager {
    pub offset: usize,
}

impl OffsetManager {
    /// # Examples
    /// ```
    /// use mbr_core::utils::data::OffsetManager;
    /// let manager = OffsetManager::new(Some(10));
    /// assert_eq!(manager.offset, 10);
    /// ```
    pub fn new(offset: Option<usize>) -> Self {
        OffsetManager {
            offset: offset.unwrap_or(0),
        }
    }

    /// Apply offset to QueryResult, returning sliced data from offset position
    pub fn apply_offset(&self, result: &QueryResult) -> Result<QueryResult, AppError> {
        let total_rows = result.data.rows.len();

        if self.offset > total_rows {
            return Err(Self::offset_error(self.offset, total_rows));
        }

        let rows = if self.offset == 0 {
            result.data.rows.clone()
        } else if self.offset < total_rows {
            result.data.rows[self.offset..].to_vec()
        } else {
            Vec::new()
        };

        Ok(QueryResult {
            data: QueryData {
                cols: result.data.cols.clone(),
                rows,
            },
        })
    }

    pub fn validate_offset(&self, total_records: usize) -> Result<(), AppError> {
        if self.offset > total_records {
            return Err(Self::offset_error(self.offset, total_records));
        }
        Ok(())
    }

    pub fn is_no_offset(&self) -> bool {
        self.offset == 0
    }

    pub fn remaining_records(&self, total_records: usize) -> usize {
        total_records.saturating_sub(self.offset)
    }

    fn offset_error(offset: usize, max: usize) -> AppError {
        AppError::Display(crate::error::DisplayError::Pagination(format!(
            "Offset {} is out of range (max: {})",
            offset, max
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::Column;
    use serde_json::json;

    fn test_columns() -> Vec<Column> {
        vec![
            Column {
                name: "id".into(),
                display_name: "ID".into(),
                base_type: "type/Integer".into(),
            },
            Column {
                name: "name".into(),
                display_name: "Name".into(),
                base_type: "type/Text".into(),
            },
        ]
    }

    fn test_query_result() -> QueryResult {
        QueryResult {
            data: QueryData {
                cols: test_columns(),
                rows: vec![
                    vec![json!(1), json!("Alice")],
                    vec![json!(2), json!("Bob")],
                    vec![json!(3), json!("Charlie")],
                    vec![json!(4), json!("David")],
                    vec![json!(5), json!("Eve")],
                ],
            },
        }
    }

    #[test]
    fn test_remaining_records() {
        assert_eq!(OffsetManager::new(Some(2)).remaining_records(5), 3);
        assert_eq!(OffsetManager::new(Some(5)).remaining_records(5), 0);
        assert_eq!(OffsetManager::new(Some(10)).remaining_records(5), 0);
    }

    #[test]
    fn test_apply_offset() {
        let original = test_query_result();

        // offset 0: all data
        let result = OffsetManager::new(Some(0)).apply_offset(&original).unwrap();
        assert_eq!(result.data.rows.len(), 5);

        // offset 2: from 3rd row
        let result = OffsetManager::new(Some(2)).apply_offset(&original).unwrap();
        assert_eq!(result.data.rows.len(), 3);
        assert_eq!(result.data.rows[0][0], json!(3));

        // offset at end: empty
        let result = OffsetManager::new(Some(5)).apply_offset(&original).unwrap();
        assert_eq!(result.data.rows.len(), 0);
        assert_eq!(result.data.cols.len(), 2);

        // offset out of range: error
        assert!(
            OffsetManager::new(Some(10))
                .apply_offset(&original)
                .is_err()
        );
    }
}
