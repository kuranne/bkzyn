use std::process::Command;

/// Displays the git log of snapshots.
pub fn run(
    paths: &crate::AppPaths,
    _dry_run: bool,
    _verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = paths.repo.join("data");

    if !data_dir.exists() {
        return Err("Data directory does not exist. Run setup or backup first.".into());
    }

    // Run git log with pretty formatting
    let status = Command::new("git")
        .current_dir(&data_dir)
        .arg("log")
        .arg("--color=always")
        .arg("--pretty=format:%C(yellow)%h%Creset - %C(cyan)%ad%Creset - %s %C(green)(%cr)%Creset")
        .arg("--date=short")
        .status()?;

    if !status.success() {
        return Err("Failed to view snapshot log.".into());
    }

    Ok(())
}
