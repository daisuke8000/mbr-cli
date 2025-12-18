//! Progress display utilities for long-running operations

use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

// Constants for display configuration
const SPINNER_UPDATE_INTERVAL_MS: u64 = 100;
const CLEAR_LINE_WIDTH: usize = 100;

/// Simple spinner to show progress of asynchronous operations
pub struct ProgressSpinner {
    message: String,
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl ProgressSpinner {
    /// Create new progress spinner with message
    pub fn new(message: String) -> Self {
        let running = Arc::new(AtomicBool::new(false));
        Self {
            message,
            running,
            handle: None,
        }
    }

    /// Start spinner
    pub fn start(&mut self) {
        self.running.store(true, Ordering::Relaxed);
        let running = Arc::clone(&self.running);
        let message = self.message.clone();

        let handle = thread::spawn(move || {
            let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let mut index = 0;

            while running.load(Ordering::Relaxed) {
                print!("\r{} {}", spinner_chars[index], message);
                let _ = io::stdout().flush(); // Ignore flush errors to continue operation

                index = (index + 1) % spinner_chars.len();
                thread::sleep(Duration::from_millis(SPINNER_UPDATE_INTERVAL_MS));
            }

            // Clear line properly for emoji support
            print!("\r{:<width$}\r", "", width = CLEAR_LINE_WIDTH);
            let _ = io::stdout().flush(); // Ignore flush errors to continue operation
        });

        self.handle = Some(handle);
    }

    /// Stop spinner and display completion message
    pub fn stop(&mut self, completion_message: Option<&str>) {
        self.running.store(false, Ordering::Relaxed);

        if let Some(handle) = self.handle.take() {
            let _ = handle.join(); // Ignore thread join errors
        }

        if let Some(msg) = completion_message {
            // Add space before emoji to prevent terminal clipping
            println!(" {}", msg);
            let _ = io::stdout().flush(); // Ignore flush errors
        }
    }

    /// Update running spinner message
    pub fn update_message(&mut self, message: String) {
        self.message = message;
    }
}

impl Drop for ProgressSpinner {
    fn drop(&mut self) {
        self.stop(None);
    }
}

/// Progress tracker for multi-step operations
pub struct ProgressTracker {
    steps: Vec<String>,
    current_step: usize,
    total_steps: usize,
}

impl ProgressTracker {
    /// Create new progress tracker with predefined steps
    pub fn new(steps: Vec<String>) -> Self {
        let total_steps = steps.len();
        Self {
            steps,
            current_step: 0,
            total_steps,
        }
    }

    /// Start next step and display progress
    pub fn next_step(&mut self) -> Option<String> {
        if self.current_step >= self.total_steps {
            return None;
        }

        let step_name = &self.steps[self.current_step];
        let progress = format!(
            "[{}/{}] {}",
            self.current_step + 1,
            self.total_steps,
            step_name
        );

        println!("{}", progress);
        self.current_step += 1;

        Some(step_name.clone())
    }

    /// Get current progress percentage
    pub fn progress_percent(&self) -> f32 {
        if self.total_steps == 0 {
            100.0
        } else {
            (self.current_step as f32 / self.total_steps as f32) * 100.0
        }
    }

    /// Check if all steps are completed
    pub fn is_complete(&self) -> bool {
        self.current_step >= self.total_steps
    }
}

/// Display simple progress bar
pub fn show_progress_bar(current: usize, total: usize, width: usize) {
    if total == 0 {
        return;
    }

    let progress = current as f32 / total as f32;
    let filled = (progress * width as f32) as usize;
    let empty = width.saturating_sub(filled);

    print!("\r[");
    print!("{}", "█".repeat(filled));
    print!("{}", "░".repeat(empty));
    print!("] {:.1}% ({}/{})", progress * 100.0, current, total);

    let _ = io::stdout().flush(); // Ignore flush errors

    if current == total {
        println!(); // New line on completion
    }
}

/// Display operation status with color output
pub fn display_status(operation: &str, status: OperationStatus) {
    let (symbol, message) = match status {
        OperationStatus::InProgress => ("⏳", format!("In progress: {}", operation)),
        OperationStatus::Success => ("✅", format!("Completed: {}", operation)),
        OperationStatus::Warning => ("⚠️", format!("Warning: {}", operation)),
        OperationStatus::Error => ("❌", format!("Error: {}", operation)),
    };

    // Add space before emoji to prevent terminal clipping
    println!(" {} {}", symbol, message);
}

/// Display authentication result with consistent formatting
pub fn display_auth_result<T, E: std::fmt::Display>(result: Result<T, E>, success_message: &str) {
    match result {
        Ok(_) => println!("✅ {}", success_message),
        Err(e) => println!("❌ Authentication failed: {}", e),
    }
}

/// Display operation result with consistent formatting
pub fn display_operation_result<T, E: std::fmt::Display>(
    result: Result<T, E>,
    success_message: &str,
    error_prefix: &str,
) {
    match result {
        Ok(_) => println!("✅ {}", success_message),
        Err(e) => println!("❌ {}: {}", error_prefix, e),
    }
}

/// Common error message templates
pub mod error_messages {
    pub const AUTHENTICATION_FAILED: &str = "Authentication failed";
    pub const CONNECTION_FAILED: &str = "Connection failed";
    pub const INVALID_INPUT: &str = "Invalid input";
    pub const RESOURCE_NOT_FOUND: &str = "Resource not found";
    pub const PERMISSION_DENIED: &str = "Permission denied";
    pub const TIMEOUT: &str = "Operation timed out";
    pub const INVALID_CONFIGURATION: &str = "Invalid configuration";

    /// Format error message with context
    pub fn with_context(template: &str, context: &str) -> String {
        format!("{}: {}", template, context)
    }
}

/// Types of operation status
#[derive(Debug, Clone)]
pub enum OperationStatus {
    InProgress,
    Success,
    Warning,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_tracker_creation() {
        let steps = vec![
            "Authentication".to_string(),
            "Data retrieval".to_string(),
            "Result display".to_string(),
        ];
        let tracker = ProgressTracker::new(steps.clone());

        assert_eq!(tracker.total_steps, 3);
        assert_eq!(tracker.current_step, 0);
        assert_eq!(tracker.progress_percent(), 0.0);
        assert!(!tracker.is_complete());
    }

    #[test]
    fn test_progress_tracker_steps() {
        let steps = vec!["Step 1".to_string(), "Step 2".to_string()];
        let mut tracker = ProgressTracker::new(steps);

        // First step
        let step1 = tracker.next_step();
        assert_eq!(step1, Some("Step 1".to_string()));
        assert_eq!(tracker.progress_percent(), 50.0);
        assert!(!tracker.is_complete());

        // Second step
        let step2 = tracker.next_step();
        assert_eq!(step2, Some("Step 2".to_string()));
        assert_eq!(tracker.progress_percent(), 100.0);
        assert!(tracker.is_complete());

        // No more steps
        let step3 = tracker.next_step();
        assert_eq!(step3, None);
    }

    #[test]
    fn test_progress_tracker_empty() {
        let tracker = ProgressTracker::new(vec![]);
        assert_eq!(tracker.progress_percent(), 100.0);
        assert!(tracker.is_complete());
    }

}
