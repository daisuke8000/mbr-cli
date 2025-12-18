use crate::api::models::{Column, QueryResult};
use crate::error::AppError;

/// Memory usage estimation and chunk processing utilities
pub struct MemoryEstimator;

impl MemoryEstimator {
    /// Estimate memory usage of query result (in MB)
    pub fn estimate_query_result_memory(result: &QueryResult) -> usize {
        let cols_memory = Self::estimate_columns_memory(&result.data.cols);
        let rows_memory = Self::estimate_rows_memory(&result.data.rows, &result.data.cols);

        // Consider additional overhead (structs, string management, etc.)
        let overhead = (cols_memory + rows_memory) / 10; // 10% overhead

        Self::bytes_to_mb(cols_memory + rows_memory + overhead)
    }

    /// Estimate memory usage of column information
    fn estimate_columns_memory(columns: &[Column]) -> usize {
        columns
            .iter()
            .map(|col| {
                col.name.len() + col.display_name.len() + col.base_type.len() + 64 // Struct overhead
            })
            .sum()
    }

    /// Estimate memory usage of row data
    fn estimate_rows_memory(rows: &[Vec<serde_json::Value>], columns: &[Column]) -> usize {
        if rows.is_empty() {
            return 0;
        }

        // Estimate average size from sample rows
        let sample_size = 100.min(rows.len());
        let sample_memory: usize = rows
            .iter()
            .take(sample_size)
            .map(|row| Self::estimate_row_memory(row))
            .sum();

        let avg_row_memory = if sample_size > 0 {
            sample_memory / sample_size
        } else {
            0
        };

        // Estimate for all rows
        avg_row_memory * rows.len() + columns.len() * 8 * rows.len() // Vec<Value> pointer overhead
    }

    /// Estimate memory usage of a single row
    fn estimate_row_memory(row: &[serde_json::Value]) -> usize {
        row.iter().map(Self::estimate_value_memory).sum()
    }

    /// Estimate memory usage of JSON value
    fn estimate_value_memory(value: &serde_json::Value) -> usize {
        match value {
            serde_json::Value::Null => 8,
            serde_json::Value::Bool(_) => 8,
            serde_json::Value::Number(_) => 24, // f64 + overhead
            serde_json::Value::String(s) => s.len() + 24, // String + String struct
            serde_json::Value::Array(arr) => {
                arr.iter().map(Self::estimate_value_memory).sum::<usize>() + 24
            }
            serde_json::Value::Object(obj) => {
                obj.iter()
                    .map(|(k, v)| k.len() + Self::estimate_value_memory(v))
                    .sum::<usize>()
                    + 32
            }
        }
    }

    /// Convert bytes to MB
    fn bytes_to_mb(bytes: usize) -> usize {
        (bytes / 1024 / 1024).max(1) // Minimum 1MB
    }

    /// Memory limit check
    pub fn is_within_memory_limit(result: &QueryResult, limit_mb: usize) -> bool {
        let estimated_mb = Self::estimate_query_result_memory(result);
        estimated_mb <= limit_mb
    }

    /// Calculate safe chunk size
    pub fn calculate_safe_chunk_size(
        result: &QueryResult,
        memory_limit_mb: usize,
    ) -> Result<usize, AppError> {
        let total_rows = result.data.rows.len();

        if total_rows == 0 {
            return Ok(0);
        }

        let estimated_total_mb = Self::estimate_query_result_memory(result);

        if estimated_total_mb <= memory_limit_mb {
            return Ok(total_rows); // Can process all data at once
        }

        // Calculate chunk size
        let ratio = memory_limit_mb as f64 / estimated_total_mb as f64;
        let chunk_size = ((total_rows as f64 * ratio) as usize).max(1);

        Ok(chunk_size.min(5000)) // Maximum 5000 row chunks
    }

    /// Split data into memory-efficient chunks
    pub fn create_memory_efficient_chunks<T>(
        data: &[T],
        memory_limit_mb: usize,
        estimate_fn: impl Fn(&[T]) -> usize,
    ) -> Vec<&[T]> {
        if data.is_empty() {
            return vec![];
        }

        let mut chunks = vec![];
        let mut start = 0;
        let target_size_bytes = memory_limit_mb * 1024 * 1024;

        while start < data.len() {
            let mut end = start + 1;

            // Find safe chunk size
            while end <= data.len() {
                let chunk = &data[start..end];
                let estimated_bytes = estimate_fn(chunk);

                if estimated_bytes > target_size_bytes && end > start + 1 {
                    end -= 1;
                    break;
                }

                if end == data.len() {
                    break;
                }

                end += 1;
            }

            chunks.push(&data[start..end]);
            start = end;
        }

        chunks
    }

    /// Chunk splitting for query results
    pub fn chunk_query_result(
        result: &QueryResult,
        memory_limit_mb: usize,
    ) -> Result<Vec<QueryResult>, AppError> {
        let chunk_size = Self::calculate_safe_chunk_size(result, memory_limit_mb)?;

        if chunk_size >= result.data.rows.len() {
            return Ok(vec![result.clone()]);
        }

        let mut chunks = vec![];

        for chunk_rows in result.data.rows.chunks(chunk_size) {
            let chunk_result = QueryResult {
                data: crate::api::models::QueryData {
                    cols: result.data.cols.clone(),
                    rows: chunk_rows.to_vec(),
                },
            };
            chunks.push(chunk_result);
        }

        Ok(chunks)
    }

    /// Detailed memory usage report
    pub fn generate_memory_report(result: &QueryResult) -> MemoryReport {
        let cols_memory = Self::estimate_columns_memory(&result.data.cols);
        let rows_memory = Self::estimate_rows_memory(&result.data.rows, &result.data.cols);
        let total_memory = cols_memory + rows_memory;

        MemoryReport {
            columns_memory_mb: Self::bytes_to_mb(cols_memory),
            rows_memory_mb: Self::bytes_to_mb(rows_memory),
            total_memory_mb: Self::bytes_to_mb(total_memory),
            row_count: result.data.rows.len(),
            column_count: result.data.cols.len(),
            avg_row_size_bytes: if !result.data.rows.is_empty() {
                rows_memory / result.data.rows.len()
            } else {
                0
            },
        }
    }
}

/// Memory usage report
#[derive(Debug, Clone)]
pub struct MemoryReport {
    pub columns_memory_mb: usize,
    pub rows_memory_mb: usize,
    pub total_memory_mb: usize,
    pub row_count: usize,
    pub column_count: usize,
    pub avg_row_size_bytes: usize,
}

impl MemoryReport {
    /// Display report as string
    pub fn to_display_string(&self) -> String {
        format!(
            "📊 Memory Usage Report:\n\
             ├─ Total: {}MB\n\
             ├─ Columns: {}MB ({} columns)\n\
             ├─ Rows: {}MB ({} rows)\n\
             └─ Avg row size: {} bytes",
            self.total_memory_mb,
            self.columns_memory_mb,
            self.column_count,
            self.rows_memory_mb,
            self.row_count,
            self.avg_row_size_bytes
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::{Column, QueryData, QueryResult};
    use serde_json::json;

    fn create_test_query_result(rows: usize, cols: usize) -> QueryResult {
        let columns: Vec<Column> = (0..cols)
            .map(|i| Column {
                name: format!("col_{}", i),
                display_name: format!("Column {}", i),
                base_type: "type/Text".to_string(),
            })
            .collect();

        let rows_data: Vec<Vec<serde_json::Value>> = (0..rows)
            .map(|i| {
                (0..cols)
                    .map(|j| json!(format!("row_{}_col_{}", i, j)))
                    .collect()
            })
            .collect();

        QueryResult {
            data: QueryData {
                cols: columns,
                rows: rows_data,
            },
        }
    }

    #[test]
    fn test_estimate_value_memory() {
        assert_eq!(MemoryEstimator::estimate_value_memory(&json!(null)), 8);
        assert_eq!(MemoryEstimator::estimate_value_memory(&json!(true)), 8);
        assert_eq!(MemoryEstimator::estimate_value_memory(&json!(42)), 24);

        let string_value = json!("hello");
        assert_eq!(
            MemoryEstimator::estimate_value_memory(&string_value),
            5 + 24 // "hello".len() + String overhead
        );
    }

    #[test]
    fn test_estimate_query_result_memory() {
        let result = create_test_query_result(100, 5);
        let memory_mb = MemoryEstimator::estimate_query_result_memory(&result);
        assert!(memory_mb > 0);
        assert!(memory_mb < 100); // Within reasonable range
    }

    #[test]
    fn test_memory_limit_check() {
        let small_result = create_test_query_result(10, 3);
        assert!(MemoryEstimator::is_within_memory_limit(&small_result, 100));

        let large_result = create_test_query_result(10000, 50);
        assert!(!MemoryEstimator::is_within_memory_limit(&large_result, 1));
    }

    #[test]
    fn test_calculate_safe_chunk_size() {
        let result = create_test_query_result(1000, 10);
        let chunk_size = MemoryEstimator::calculate_safe_chunk_size(&result, 50);

        assert!(chunk_size.is_ok());
        let size = chunk_size.unwrap();
        assert!(size > 0);
        assert!(size <= 5000); // Maximum chunk size limit
    }

    #[test]
    fn test_chunk_query_result() {
        let result = create_test_query_result(100, 5);
        let chunks = MemoryEstimator::chunk_query_result(&result, 1);

        assert!(chunks.is_ok());
        let chunk_vec = chunks.unwrap();
        assert!(!chunk_vec.is_empty());

        // Verify that total rows in all chunks match original row count
        let total_rows: usize = chunk_vec.iter().map(|c| c.data.rows.len()).sum();
        assert_eq!(total_rows, 100);
    }

    #[test]
    fn test_memory_report() {
        let result = create_test_query_result(50, 3);
        let report = MemoryEstimator::generate_memory_report(&result);

        assert_eq!(report.row_count, 50);
        assert_eq!(report.column_count, 3);
        assert!(report.total_memory_mb > 0);

        let display_string = report.to_display_string();
        assert!(display_string.contains("Memory Usage Report"));
        assert!(display_string.contains("50 rows"));
        assert!(display_string.contains("3 columns"));
    }

    #[test]
    fn test_bytes_to_mb() {
        assert_eq!(MemoryEstimator::bytes_to_mb(0), 1); // Minimum value
        assert_eq!(MemoryEstimator::bytes_to_mb(1024 * 1024), 1); // 1MB
        assert_eq!(MemoryEstimator::bytes_to_mb(2 * 1024 * 1024), 2); // 2MB
    }
}
