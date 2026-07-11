//! CLI UI manager and output formatting.

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const GREEN: &str = "\x1b[1;32m";
const YELLOW: &str = "\x1b[1;33m";
const BLUE: &str = "\x1b[1;34m";
const MAGENTA: &str = "\x1b[1;35m";
const CYAN: &str = "\x1b[1;36m";
const RED: &str = "\x1b[1;31m";

use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// CLI Manager to handle styled output.
pub struct CliManager {
    verbose: bool,
    spinner: Option<ProgressBar>,
}

impl CliManager {
    /// Creates a new CliManager.
    pub fn new(verbose: bool) -> Self {
        let spinner = if !verbose {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                    .template("{spinner:.green} {msg}")
                    .unwrap_or_else(|_| ProgressStyle::default_spinner()),
            );
            pb.enable_steady_tick(Duration::from_millis(100));
            Some(pb)
        } else {
            None
        };

        Self { verbose, spinner }
    }

    /// Sets the current progress message (only visible in non-verbose mode)
    fn progress(&self, msg: &str) {
        if let Some(pb) = &self.spinner {
            pb.set_message(msg.to_string());
        }
    }

    /// Prints a status message if verbose is enabled, or updates the spinner.
    pub fn status(&self, status: &str, title: &str, desc: &str) {
        if self.verbose {
            let color = match status {
                "INFO" => CYAN,
                "SKIP" => YELLOW,
                "COPY" => BLUE,
                "LINK" => MAGENTA,
                "BACKUP" => GREEN,
                "WARN" => RED,
                "DELETE" => RED,
                _ => RESET,
            };
            println!(
                "{}[ {}{}{}{} ] {}{}{}: {}",
                BOLD, color, status, RESET, BOLD, BOLD, title, RESET, desc
            );
        } else {
            // Update the spinner description silently
            self.progress(desc);
        }
    }

    /// Prints a done/success message.
    pub fn done(&self, desc: &str) {
        let msg = format!(
            "{}[ {}DONE{}{} ] {}{}{}",
            BOLD, GREEN, RESET, BOLD, BOLD, desc, RESET
        );
        if let Some(pb) = &self.spinner {
            pb.finish_and_clear();
        }
        println!("{}", msg);
    }

    /// Prints a warning message without breaking the spinner.
    pub fn warn(&self, title: &str, desc: &str) {
        let msg = format!(
            "{}[ {}WARN{}{} ] {}{}{}: {}",
            BOLD, RED, RESET, BOLD, BOLD, title, RESET, desc
        );
        if let Some(pb) = &self.spinner {
            pb.println(msg);
        } else {
            println!("{}", msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let manager = CliManager::new(true);
        assert_eq!(manager.verbose, true);
        assert!(manager.spinner.is_none());

        let manager_quiet = CliManager::new(false);
        assert_eq!(manager_quiet.verbose, false);
        assert!(manager_quiet.spinner.is_some());
    }

    #[test]
    fn test_status() {
        let manager = CliManager::new(true);
        manager.status("INFO", "Test", "Test Info status");
        manager.status("SKIP", "Test", "Test Skip status");

        let quiet_manager = CliManager::new(false);
        quiet_manager.status(
            "INFO",
            "Test Quiet",
            "This should not print, just update spinner",
        );
    }

    #[test]
    fn test_done() {
        let manager = CliManager::new(true);
        manager.done("Test done action");
    }

    #[test]
    fn test_warn() {
        let manager = CliManager::new(true);
        manager.warn("Test", "Test warning message");
    }
}
