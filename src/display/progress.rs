//! Progress display utilities for long-running operations

use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

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
                io::stdout().flush().unwrap_or_default();

                index = (index + 1) % spinner_chars.len();
                thread::sleep(Duration::from_millis(100));
            }

            // Clear line properly for emoji support
            print!("\r{:<100}\r", "");
            io::stdout().flush().unwrap_or_default();
        });

        self.handle = Some(handle);
    }

    /// Stop spinner and display completion message
    pub fn stop(&mut self, completion_message: Option<&str>) {
        self.running.store(false, Ordering::Relaxed);

        if let Some(handle) = self.handle.take() {
            handle.join().unwrap_or_default();
        }

        if let Some(msg) = completion_message {
            // Add space before emoji to prevent terminal clipping
            println!(" {}", msg);
            io::stdout().flush().unwrap_or_default();
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

    io::stdout().flush().unwrap_or_default();

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

    #[test]
    fn test_operation_status_display() {
        // Test creating and using different status types
        let statuses = [
            OperationStatus::InProgress,
            OperationStatus::Success,
            OperationStatus::Warning,
            OperationStatus::Error,
        ];

        for status in statuses {
            display_status("Test operation", status);
        }
    }

    #[test]
    fn test_progress_spinner_creation() {
        let spinner = ProgressSpinner::new("Testing...".to_string());
        assert_eq!(spinner.message, "Testing...");
        assert!(!spinner.running.load(Ordering::Relaxed));
    }
}
