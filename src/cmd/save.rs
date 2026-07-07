use std::process::Command;

/// Saves (commits and optionally pushes) changes to the backup repository
pub fn run(
    paths: &crate::AppPaths,
    category: Option<&str>,
    message: Option<&str>,
    dry_run: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ui = crate::cli::CliManager::new(verbose);

    let commit_message = message.unwrap_or("Update configurations");

    ui.status(
        "INFO",
        "Git",
        &format!(
            "Saving changes to repository at {}...",
            paths.repo.display()
        ),
    );

    if !dry_run {
        let backup_repo = paths.repo.join("data");
        
        // Auto-initialize git if the data folder was wiped
        if !backup_repo.join(".git").exists() {
            if !backup_repo.exists() {
                std::fs::create_dir_all(&backup_repo)?;
            }
            Command::new("git")
                .current_dir(&backup_repo)
                .arg("init")
                .status()?;
            ui.status("INFO", "Git", "Initialized new git repository in data/");
        }

        // 1. Determine current branch
        let current_branch_out = Command::new("git")
            .current_dir(&backup_repo)
            .args(["branch", "--show-current"])
            .output()?;
        let current_branch = String::from_utf8_lossy(&current_branch_out.stdout).trim().to_string();

        let target_branch = "backup";

        // 2. Checkout or create target branch
        ui.status("INFO", "Git", &format!("Switching to {} branch...", target_branch));
        let checkout_status = Command::new("git")
            .current_dir(&backup_repo)
            .args(["checkout", target_branch])
            .status()?;
        
        if !checkout_status.success() {
            // Branch doesn't exist locally, create it
            Command::new("git")
                .current_dir(&backup_repo)
                .args(["checkout", "-b", target_branch])
                .status()?;
        }

        // 3. Git add
        let add_path = if let Some(cat) = category {
            cat.to_string()
        } else {
            ".".to_string()
        };

        let add_status = Command::new("git")
            .current_dir(&backup_repo)
            .args(["add", &add_path])
            .status()?;

        if !add_status.success() {
            return Err("Failed to execute `git add`".into());
        }

        // 4. Git commit
        let commit_status = Command::new("git")
            .current_dir(&backup_repo)
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

        // 5. Git push
        ui.status("INFO", "Git", &format!("Pushing {} to origin...", target_branch));
        let push_status = Command::new("git")
            .current_dir(&backup_repo)
            .args(["push", "-u", "origin", target_branch])
            .status()?;
        
        if !push_status.success() {
            ui.warn("Git", "Failed to push to origin. (Are you offline or lacking permissions?)");
        }

        // 6. Return to original branch
        if !current_branch.is_empty() && current_branch != target_branch {
            ui.status("INFO", "Git", &format!("Switching back to {}...", current_branch));
            Command::new("git")
                .current_dir(&backup_repo)
                .args(["checkout", &current_branch])
                .status()?;
        }
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
