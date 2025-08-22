// Start position specification functionality using --offset option

use crate::api::models::{QueryData, QueryResult};
use crate::error::AppError;

/// Struct to manage offset functionality
///
/// Provides functionality to display from the start position specified by --offset option.
/// When used in combination with pagination, it enables display from arbitrary positions in large datasets.
#[derive(Debug, Clone)]
pub struct OffsetManager {
    /// Start position (0-based)
    pub offset: usize,
}

impl OffsetManager {
    /// Create new OffsetManager
    ///
    /// # Arguments
    /// * `offset` - Start position (0-based, defaults to 0 if None)
    ///
    /// # Examples
    /// ```
    /// use mbr_cli::utils::data::OffsetManager;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let no_offset = OffsetManager::new(None)?;
    /// let with_offset = OffsetManager::new(Some(10))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(offset: Option<usize>) -> Result<Self, AppError> {
        let offset = offset.unwrap_or(0);
        Ok(OffsetManager { offset })
    }

    /// Apply offset to QueryResult
    ///
    /// # Arguments
    /// * `result` - Original QueryResult
    ///
    /// # Returns
    /// New QueryResult starting from offset position
    ///
    /// # Errors
    /// Returns error if offset is out of data range
    ///
    /// # Performance
    /// Performs efficient slicing for large datasets
    pub fn apply_offset(&self, result: &QueryResult) -> Result<QueryResult, AppError> {
        let total_rows = result.data.rows.len();

        // Check if offset is out of range
        if self.offset > total_rows {
            return Err(AppError::Display(crate::error::DisplayError::Pagination(
                format!(
                    "Offset {} is out of range (max: {})",
                    self.offset, total_rows
                ),
            )));
        }

        // Optimization for offset 0 (avoid data copying)
        if self.offset == 0 {
            return Ok(QueryResult {
                data: QueryData {
                    cols: result.data.cols.clone(),
                    rows: result.data.rows.clone(),
                },
            });
        }

        // Extract data from offset position (efficient slicing)
        let offset_rows = if self.offset < total_rows {
            result.data.rows[self.offset..].to_vec()
        } else {
            Vec::new() // Empty if offset is at end
        };

        Ok(QueryResult {
            data: QueryData {
                cols: result.data.cols.clone(),
                rows: offset_rows,
            },
        })
    }

    /// Validate if offset is valid
    ///
    /// # Arguments
    /// * `total_records` - Total record count
    ///
    /// # Errors
    /// Returns error if offset is out of range
    pub fn validate_offset(&self, total_records: usize) -> Result<(), AppError> {
        if self.offset > total_records {
            return Err(AppError::Display(crate::error::DisplayError::Pagination(
                format!(
                    "Offset {} is out of range (max: {})",
                    self.offset, total_records
                ),
            )));
        }
        Ok(())
    }

    /// Check if no offset is set
    ///
    /// # Returns
    /// true if offset is 0
    pub fn is_no_offset(&self) -> bool {
        self.offset == 0
    }

    /// Calculate remaining record count
    ///
    /// # Arguments
    /// * `total_records` - Total record count
    ///
    /// # Returns
    /// Remaining record count from offset position
    pub fn remaining_records(&self, total_records: usize) -> usize {
        if self.offset >= total_records {
            0
        } else {
            total_records - self.offset
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::Column;
    use serde_json::json;

    // Test helper function
    fn create_test_columns() -> Vec<Column> {
        vec![
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
        ]
    }

    fn create_test_query_result() -> QueryResult {
        QueryResult {
            data: QueryData {
                cols: create_test_columns(),
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
    fn test_offset_manager_new_with_none() {
        // When None is passed, offset is 0
        let manager = OffsetManager::new(None).unwrap();
        assert_eq!(manager.offset, 0);
        assert!(manager.is_no_offset());
    }

    #[test]
    fn test_offset_manager_new_with_value() {
        // When value is passed, use that value
        let manager = OffsetManager::new(Some(10)).unwrap();
        assert_eq!(manager.offset, 10);
        assert!(!manager.is_no_offset());
    }

    #[test]
    fn test_remaining_records_middle() {
        // Remaining records from middle position
        let manager = OffsetManager::new(Some(2)).unwrap();
        let remaining = manager.remaining_records(5);
        assert_eq!(remaining, 3);
    }

    #[test]
    fn test_remaining_records_end() {
        // Remaining records from end position
        let manager = OffsetManager::new(Some(5)).unwrap();
        let remaining = manager.remaining_records(5);
        assert_eq!(remaining, 0);
    }

    #[test]
    fn test_remaining_records_out_of_range() {
        // Remaining records from out-of-range position
        let manager = OffsetManager::new(Some(10)).unwrap();
        let remaining = manager.remaining_records(5);
        assert_eq!(remaining, 0);
    }

    #[test]
    fn test_is_no_offset() {
        // No offset check
        let manager_no_offset = OffsetManager::new(Some(0)).unwrap();
        assert!(manager_no_offset.is_no_offset());

        let manager_with_offset = OffsetManager::new(Some(1)).unwrap();
        assert!(!manager_with_offset.is_no_offset());
    }

    #[test]
    fn test_apply_offset_no_offset() {
        // For offset 0, return all data
        let manager = OffsetManager::new(Some(0)).unwrap();
        let original = create_test_query_result();

        let result = manager.apply_offset(&original).unwrap();

        assert_eq!(result.data.rows.len(), 5);
        assert_eq!(result.data.rows[0][0], json!(1));
        assert_eq!(result.data.rows[4][0], json!(5));
    }

    #[test]
    fn test_apply_offset_middle_position() {
        // For offset 2, from 3rd to last
        let manager = OffsetManager::new(Some(2)).unwrap();
        let original = create_test_query_result();

        let result = manager.apply_offset(&original).unwrap();

        assert_eq!(result.data.rows.len(), 3);
        assert_eq!(result.data.rows[0][0], json!(3)); // Charlie
        assert_eq!(result.data.rows[2][0], json!(5)); // Eve
    }

    #[test]
    fn test_apply_offset_last_position() {
        // For offset at last position, empty data
        let manager = OffsetManager::new(Some(5)).unwrap();
        let original = create_test_query_result();

        let result = manager.apply_offset(&original).unwrap();

        assert_eq!(result.data.rows.len(), 0);
        assert_eq!(result.data.cols.len(), 2); // Column info is preserved
    }

    #[test]
    fn test_apply_offset_out_of_range() {
        // For offset out of range, error
        let manager = OffsetManager::new(Some(10)).unwrap();
        let original = create_test_query_result();

        let result = manager.apply_offset(&original);
        assert!(result.is_err());

        if let Err(AppError::Display(crate::error::DisplayError::Pagination(msg))) = result {
            assert!(msg.contains("Offset 10 is out of range"));
        } else {
            panic!("Expected DisplayError::Pagination");
        }
    }

    #[test]
    fn test_validate_offset_valid() {
        // Valid offset
        let manager = OffsetManager::new(Some(3)).unwrap();
        let result = manager.validate_offset(5);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_offset_invalid() {
        // Invalid offset
        let manager = OffsetManager::new(Some(10)).unwrap();
        let result = manager.validate_offset(5);
        assert!(result.is_err());

        if let Err(AppError::Display(crate::error::DisplayError::Pagination(msg))) = result {
            assert!(msg.contains("Offset 10 is out of range"));
        } else {
            panic!("Expected DisplayError::Pagination");
        }
    }
}
