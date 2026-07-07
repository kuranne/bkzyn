use std::fs;
use std::process::Command;

/// Checks for differences between system configuration and the latest snapshot.
pub fn run(
    paths: &crate::AppPaths,
    _dry_run: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ui = crate::cli::CliManager::new(verbose);

    if !paths.config.exists() {
        return Err("No tracked configurations found in the repository.".into());
    }

    ui.status(
        "INFO",
        "Status",
        "Comparing system configurations against snapshot...",
    );

    let mut differences_found = false;

    for entry in fs::read_dir(&paths.config)? {
        let entry = entry?;
        let repo_path = entry.path();

        if repo_path.is_dir() {
            let app_name = entry.file_name();
            let sys_path = paths.xdg_config.join(&app_name);

            if !sys_path.exists() {
                ui.warn(
                    "Status",
                    &format!(
                        "{} is missing from system (~/.config/{})",
                        app_name.to_string_lossy(),
                        app_name.to_string_lossy()
                    ),
                );
                differences_found = true;
                continue;
            }

            // Use git diff --no-index for a nice colored output of differences
            let status = Command::new("git")
                .arg("diff")
                .arg("--no-index")
                .arg("--color=always")
                .arg(&repo_path)
                .arg(&sys_path)
                .status()?;

            if !status.success() {
                // git diff --no-index exits with 1 if differences are found
                differences_found = true;
            }
        }
    }

    if !differences_found {
        ui.done("System is fully in sync with the latest snapshot. No changes detected.");
    } else {
        println!("\nUnsaved changes detected. Run `bkzyn backup` to sync changes into the repository, then `bkzyn save` to snapshot them.");
    }

    Ok(())
}
