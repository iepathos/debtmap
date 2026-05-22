//! Shared helpers for git history integration tests.

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

pub fn setup_test_repo() -> Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().to_path_buf();

    Command::new("git")
        .arg("init")
        .current_dir(&repo_path)
        .output()?;

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()?;

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()?;

    Ok((temp_dir, repo_path))
}

pub fn create_test_file(repo_path: &Path, file_name: &str, content: &str) -> Result<PathBuf> {
    let file_path = repo_path.join(file_name);
    std::fs::write(&file_path, content)?;

    Command::new("git")
        .args(["add", file_name])
        .current_dir(repo_path)
        .output()?;

    Ok(file_path)
}

pub fn commit_with_message(repo_path: &Path, message: &str) -> Result<()> {
    Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn modify_and_commit(
    repo_path: &Path,
    file_name: &str,
    content: &str,
    message: &str,
) -> Result<()> {
    let file_path = repo_path.join(file_name);
    std::fs::write(&file_path, content)?;

    Command::new("git")
        .args(["add", file_name])
        .current_dir(repo_path)
        .output()?;

    commit_with_message(repo_path, message)
}
