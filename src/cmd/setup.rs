use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Sets up packages and copies or links configurations.
pub fn run(
    paths: &crate::AppPaths,
    dry_run: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ui = crate::cli::CliManager::new(verbose);

    // 1. Install packages from packages.toml
    let mut toml_path = paths.xdg_config.join("bkzyn").join("packages.toml");
    if !toml_path.exists() {
        toml_path = paths.config.join("bkzyn").join("packages.toml");
    }

    if toml_path.exists() {
        let content = std::fs::read_to_string(&toml_path)?;
        if let Ok(packages) = toml::from_str::<std::collections::HashMap<String, Vec<String>>>(&content) {
            if let Some(nix_pkgs) = packages.get("nix") {
                if !nix_pkgs.is_empty() && command_exists("nix-env") {
                    ui.status("INFO", "Setup", "Installing nix packages...");
                    if !dry_run {
                        Command::new("nix-env").arg("-iA").args(nix_pkgs).status()?;
                    }
                }
            }
            if let Some(brew_pkgs) = packages.get("brew") {
                if !brew_pkgs.is_empty() && command_exists("brew") {
                    ui.status("INFO", "Setup", "Installing brew packages...");
                    if !dry_run {
                        Command::new("brew").arg("install").args(brew_pkgs).status()?;
                    }
                }
            }
            if let Some(apt_pkgs) = packages.get("apt") {
                if !apt_pkgs.is_empty() && command_exists("apt-get") {
                    ui.status("INFO", "Setup", "Installing apt packages...");
                    if !dry_run {
                        Command::new("sudo").arg("apt-get").arg("install").arg("-y").args(apt_pkgs).status()?;
                    }
                }
            }
            if let Some(pacman_pkgs) = packages.get("pacman") {
                if !pacman_pkgs.is_empty() && command_exists("pacman") {
                    ui.status("INFO", "Setup", "Installing pacman packages...");
                    if !dry_run {
                        Command::new("sudo").arg("pacman").arg("-S").arg("--noconfirm").args(pacman_pkgs).status()?;
                    }
                }
            }
            if let Some(yay_pkgs) = packages.get("yay") {
                if !yay_pkgs.is_empty() && command_exists("yay") {
                    ui.status("INFO", "Setup", "Installing yay packages...");
                    if !dry_run {
                        Command::new("yay").arg("-S").arg("--noconfirm").args(yay_pkgs).status()?;
                    }
                }
            }
        } else {
            ui.warn("Setup", "Failed to parse packages.toml");
        }
    }

    // 1b. Legacy brew bundle support
    let brewfile = paths.repo.join("data").join("Brewfile");
    if brewfile.exists() {
        ui.status("INFO", "Setup", "Running brew bundle...");
        if !dry_run {
            let status = Command::new("brew")
                .arg("bundle")
                .arg(format!("--file={}", brewfile.display()))
                .status()?;
            if !status.success() {
                ui.warn(
                    "Setup",
                    &format!("brew bundle failed with status: {}", status),
                );
            }
        }
    }

    // 1c. Nix flake support
    let flake = paths.repo.join("data").join("flake.nix");
    if flake.exists() {
        ui.status("INFO", "Setup", "Applying Nix flake...");
        if !dry_run {
            let status = Command::new("nix")
                .current_dir(paths.repo.join("data"))
                .arg("profile")
                .arg("install")
                .arg(".")
                .status()?;
            if !status.success() {
                ui.warn(
                    "Setup",
                    &format!("nix profile install failed with status: {}", status),
                );
            }
        }
    }

    // 2. copy config/* to $XDG_CONFIG_HOME
    super::restore::run(paths, dry_run, verbose)?;

    // 3. add a line in global zshenv to use $ZDOTDIR for zsh
    let zshenv_path = if std::path::Path::new("/etc/zsh").exists() {
        "/etc/zsh/zshenv"
    } else {
        "/etc/zshenv"
    };

    ui.status(
        "INFO",
        "Setup",
        &format!("Checking {} for ZDOTDIR configuration...", zshenv_path),
    );

    let zshenv_content = fs::read_to_string(zshenv_path).unwrap_or_default();
    if !zshenv_content.contains("ZDOTDIR") {
        ui.status(
            "INFO",
            "Setup",
            &format!("Adding ZDOTDIR to {} (requires sudo)...", zshenv_path),
        );

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
                .arg(zshenv_path)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .spawn()?;

            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(snippet.as_bytes())?;
            }
            child.wait()?;
        }
    } else {
        ui.status(
            "SKIP",
            "Setup",
            &format!("ZDOTDIR already configured in {}.", zshenv_path),
        );
    }

    ui.done("Successful setup");
    Ok(())
}
