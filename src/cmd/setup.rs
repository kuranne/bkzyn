use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

pub fn run(dry_run: bool, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    let repo_dir = std::env::current_dir()?;

    // 1. brew bundle
    let brewfile = repo_dir.join("Brewfile");
    if brewfile.exists() {
        if verbose {
            println!("--> Running brew bundle...");
        }
        if !dry_run {
            let status = Command::new("brew")
                .arg("bundle")
                .arg(format!("--file={}", brewfile.display()))
                .status()?;
            if !status.success() {
                println!("Warning: brew bundle failed with status: {}", status);
            }
        }
    } else if verbose {
        println!("--> No Brewfile found, skipping brew bundle.");
    }

    // 2. copy config/* to $XDG_CONFIG_HOME
    super::restore::run(dry_run, verbose)?;

    // 3. add a line in /etc/zshenv to use $ZDOTDIR for zsh
    if verbose {
        println!("--> Checking /etc/zshenv for ZDOTDIR configuration...");
    }

    let zshenv_content = fs::read_to_string("/etc/zshenv").unwrap_or_default();
    if !zshenv_content.contains("ZDOTDIR") {
        if verbose {
            println!("--> Adding ZDOTDIR to /etc/zshenv (requires sudo)...");
        }

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
    } else if verbose {
        println!("--> ZDOTDIR already configured in /etc/zshenv.");
    }

    Ok(())
}
