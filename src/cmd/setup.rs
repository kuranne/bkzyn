use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

/// Sets up brew packages and copies or links configurations.
pub fn run(paths: &crate::AppPaths, dry_run: bool, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    let ui = crate::cli::CliManager::new(verbose);

    // 1. brew bundle
    let brewfile = paths.repo.join("Brewfile");
    if brewfile.exists() {
        ui.status("INFO", "Setup", "Running brew bundle...");
        if !dry_run {
            let status = Command::new("brew")
                .arg("bundle")
                .arg(format!("--file={}", brewfile.display()))
                .status()?;
            if !status.success() {
                ui.warn("Setup", &format!("brew bundle failed with status: {}", status));
            }
        }
    } else {
        ui.status("SKIP", "Setup", "No Brewfile found, skipping brew bundle.");
    }

    // 2. copy config/* to $XDG_CONFIG_HOME
    super::restore::run(paths, dry_run, verbose)?;

    // 3. add a line in /etc/zshenv to use $ZDOTDIR for zsh
    ui.status("INFO", "Setup", "Checking /etc/zshenv for ZDOTDIR configuration...");

    let zshenv_content = fs::read_to_string("/etc/zshenv").unwrap_or_default();
    if !zshenv_content.contains("ZDOTDIR") {
        ui.status("INFO", "Setup", "Adding ZDOTDIR to /etc/zshenv (requires sudo)...");

        let snippet = r#"
# --- XDG & ZDOTDIR bootstrap ---
if [[ -z "$XDG_CONFIG_HOME" ]]; then
    export XDG_CONFIG_HOME="$HOME/.config"
fi

if [[ -d "$XDG_CONFIG_HOME/zsh" ]]; then
    export ZDOTDIR="$XDG_CONFIG_HOME/zsh"
fi
"#;
        if !dry_run {
            let mut child = Command::new("sudo")
                .arg("tee")
                .arg("-a")
                .arg("/etc/zshenv")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .spawn()?;

            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(snippet.as_bytes())?;
            }
            child.wait()?;
        }
    } else {
        ui.status("SKIP", "Setup", "ZDOTDIR already configured in /etc/zshenv.");
    }

    ui.done("Successful setup");
    Ok(())
}
