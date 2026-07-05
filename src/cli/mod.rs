//! CLI UI manager and output formatting.

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const GREEN: &str = "\x1b[1;32m";
const YELLOW: &str = "\x1b[1;33m";
const BLUE: &str = "\x1b[1;34m";
const MAGENTA: &str = "\x1b[1;35m";
const CYAN: &str = "\x1b[1;36m";
const RED: &str = "\x1b[1;31m";

/// CLI Manager to handle styled output.
pub struct CliManager {
    verbose: bool,
}

impl CliManager {
    /// Creates a new CliManager.
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    /// Prints a status message if verbose is enabled.
    pub fn status(&self, status: &str, title: &str, desc: &str) {
        if self.verbose {
            let color = match status {
                "INFO" => CYAN,
                "SKIP" => YELLOW,
                "COPY" => BLUE,
                "LINK" => MAGENTA,
                "BACKUP" => GREEN,
                "WARN" => RED,
                _ => RESET,
            };
            println!("{}[ {}{}{}{} ] {}{}{}: {}", BOLD, color, status, RESET, BOLD, BOLD, title, RESET, desc);
        }
    }

    /// Prints a done/success message.
    pub fn done(&self, desc: &str) {
        println!("{}[ {}{}{}{} ] {}{}{}", BOLD, GREEN, "DONE", RESET, BOLD, BOLD, desc, RESET);
    }

    /// Prints a warning message.
    pub fn warn(&self, title: &str, desc: &str) {
        println!("{}[ {}{}{}{} ] {}{}{}: {}", BOLD, RED, "WARN", RESET, BOLD, BOLD, title, RESET, desc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let manager = CliManager::new(true);
        assert_eq!(manager.verbose, true);

        let manager_quiet = CliManager::new(false);
        assert_eq!(manager_quiet.verbose, false);
    }

    #[test]
    fn test_status() {
        // These tests primarily check that the formatting and println macros don't panic.
        let manager = CliManager::new(true);
        manager.status("INFO", "Test", "Test Info status");
        manager.status("SKIP", "Test", "Test Skip status");
        manager.status("COPY", "Test", "Test Copy status");
        manager.status("LINK", "Test", "Test Link status");
        manager.status("BACKUP", "Test", "Test Backup status");
        manager.status("WARN", "Test", "Test Warn status");
        manager.status("UNKNOWN", "Test", "Test Unknown status");

        let quiet_manager = CliManager::new(false);
        quiet_manager.status("INFO", "Test Quiet", "This should not print");
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
