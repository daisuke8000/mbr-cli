use crate::error::AppError;

/// Struct to manage pagination configuration
#[derive(Debug, Clone)]
pub struct PaginationConfig {
    pub page_size: usize,
    pub max_records: Option<usize>,
    pub offset: usize,
}

/// Struct to manage pagination state
#[derive(Debug, Clone)]
pub struct PaginationState {
    pub current_offset: usize,
    pub total_records: usize,
    pub is_last_page: bool,
    pub is_first_page: bool,
}

/// Display mode
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayMode {
    Paginated,
    Full,
    Interactive,
}

/// Pagination manager - Core class for pagination operations
#[derive(Debug)]
pub struct PaginationManager {
    config: PaginationConfig,
    state: PaginationState,
    mode: DisplayMode,
}

impl PaginationManager {
    /// Create new PaginationManager
    pub fn new(
        page_size: usize,
        total_records: usize,
        offset: usize,
        mode: DisplayMode,
    ) -> Result<Self, AppError> {
        if page_size == 0 {
            return Err(AppError::Display(crate::error::DisplayError::Pagination(
                "Page size must be greater than 0".to_string(),
            )));
        }

        if offset > total_records {
            return Err(AppError::Display(crate::error::DisplayError::Pagination(
                format!("Offset {} exceeds total records {}", offset, total_records),
            )));
        }

        let config = PaginationConfig {
            page_size,
            max_records: None,
            offset,
        };

        let state = PaginationState {
            current_offset: offset,
            total_records,
            is_first_page: offset == 0,
            is_last_page: offset + page_size >= total_records,
        };

        Ok(PaginationManager {
            config,
            state,
            mode,
        })
    }

    /// Move to next page
    pub fn next_page(&mut self) -> bool {
        if self.state.is_last_page {
            return false;
        }

        self.state.current_offset += self.config.page_size;
        self.update_state();
        true
    }

    /// Move to previous page
    pub fn previous_page(&mut self) -> bool {
        if self.state.is_first_page {
            return false;
        }

        self.state.current_offset = self
            .state
            .current_offset
            .saturating_sub(self.config.page_size);
        self.update_state();
        true
    }

    /// Update pagination state
    fn update_state(&mut self) {
        self.state.is_first_page = self.state.current_offset == 0;
        self.state.is_last_page =
            self.state.current_offset + self.config.page_size >= self.state.total_records;
    }

    /// Get current page information
    pub fn get_page_info(&self) -> (usize, usize, usize, usize) {
        let current_page = (self.state.current_offset / self.config.page_size) + 1;
        let total_pages = self
            .state
            .total_records
            .div_ceil(self.config.page_size)
            .max(1);
        let start_item = self.state.current_offset + 1;
        let end_item =
            (self.state.current_offset + self.config.page_size).min(self.state.total_records);

        (current_page, total_pages, start_item, end_item)
    }

    /// Get data slice for display
    pub fn get_page_slice<'a, T>(&self, data: &'a [T]) -> &'a [T] {
        let start = self.state.current_offset.min(data.len());
        let end = (start + self.config.page_size).min(data.len());
        &data[start..end]
    }

    /// Generate pagination information string
    pub fn get_pagination_info(&self) -> String {
        let (current_page, total_pages, start_item, end_item) = self.get_page_info();

        if self.state.total_records == 0 {
            return "No records found".to_string();
        }

        format!(
            "Showing {}-{} of {} records (Page {} of {})",
            start_item, end_item, self.state.total_records, current_page, total_pages
        )
    }

    /// Get configuration
    pub fn get_config(&self) -> &PaginationConfig {
        &self.config
    }

    /// Get state
    pub fn get_state(&self) -> &PaginationState {
        &self.state
    }

    /// Get display mode
    pub fn get_mode(&self) -> &DisplayMode {
        &self.mode
    }
}

impl Default for PaginationManager {
    fn default() -> Self {
        Self::new(20, 0, 0, DisplayMode::Paginated)
            .expect("Failed to create default PaginationManager with valid parameters")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_manager_creation() {
        let manager = PaginationManager::new(10, 100, 0, DisplayMode::Paginated)
            .expect("Failed to create PaginationManager for test");
        assert_eq!(manager.config.page_size, 10);
        assert_eq!(manager.state.total_records, 100);
        assert_eq!(manager.state.current_offset, 0);
        assert!(manager.state.is_first_page);
        assert!(!manager.state.is_last_page);
    }

    #[test]
    fn test_page_navigation() {
        let mut manager = PaginationManager::new(10, 25, 0, DisplayMode::Paginated)
            .expect("Failed to create PaginationManager for navigation test");

        // First page
        let (current, total, start, end) = manager.get_page_info();
        assert_eq!(current, 1);
        assert_eq!(total, 3);
        assert_eq!(start, 1);
        assert_eq!(end, 10);

        // Go to the next page
        assert!(manager.next_page());
        let (current, total, start, end) = manager.get_page_info();
        assert_eq!(current, 2);
        assert_eq!(total, 3);
        assert_eq!(start, 11);
        assert_eq!(end, 20);

        // Go to the last page
        assert!(manager.next_page());
        let (current, total, start, end) = manager.get_page_info();
        assert_eq!(current, 3);
        assert_eq!(total, 3);
        assert_eq!(start, 21);
        assert_eq!(end, 25);
        assert!(manager.state.is_last_page);

        // Cannot go beyond the last page
        assert!(!manager.next_page());
    }

    #[test]
    fn test_invalid_parameters() {
        // Page size cannot be 0
        assert!(PaginationManager::new(0, 100, 0, DisplayMode::Paginated).is_err());

        // Offset cannot exceed total records
        assert!(PaginationManager::new(10, 100, 150, DisplayMode::Paginated).is_err());
    }
}
