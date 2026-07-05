use std::process::Command;

/// Saves (commits and optionally pushes) changes to the backup repository
pub fn run(
    paths: &crate::AppPaths,
    message: Option<&String>,
    dry_run: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ui = crate::cli::CliManager::new(verbose);

    let commit_message = message
        .map(|s| s.as_str())
        .unwrap_or("Update configurations");

    ui.status(
        "INFO",
        "Git",
        &format!(
            "Saving changes to repository at {}...",
            paths.repo.display()
        ),
    );

    if !dry_run {
        // 1. Git add
        let add_status = Command::new("git")
            .current_dir(&paths.repo)
            .args(["add", "."])
            .status()?;

        if !add_status.success() {
            return Err("Failed to execute `git add`".into());
        }

        // 2. Git commit
        let commit_status = Command::new("git")
            .current_dir(&paths.repo)
            .args(["commit", "-m", commit_message])
            .status()?;

        if !commit_status.success() {
            ui.warn(
                "Git",
                "Git commit returned a non-zero status. (Maybe there were no changes to commit?)",
            );
        } else {
            ui.status(
                "INFO",
                "Git",
                &format!("Committed with message: \"{}\"", commit_message),
            );
        }

        // Optional: you could add a `git push` here if the user wanted it
    } else {
        ui.status(
            "SKIP",
            "Git",
            &format!(
                "Dry run - would have committed with message: \"{}\"",
                commit_message
            ),
        );
    }

    ui.done("Successfully saved changes");
    Ok(())
}
