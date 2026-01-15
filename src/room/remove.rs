#![allow(dead_code)]

use std::path::Path;

use thiserror::Error;

use crate::git::command::{CommandError, GitCommand};
use crate::state::RoomsState;

#[derive(Error, Debug)]
pub enum RemoveRoomError {
    #[error("room '{0}' not found")]
    NotFound(String),

    #[error("failed to check worktree status: {0}")]
    StatusCheck(String),

    #[error("failed to remove worktree: {0}")]
    WorktreeRemoval(String),

    #[error("git command failed: {0}")]
    GitError(#[from] CommandError),
}

/// Information about uncommitted changes in a worktree.
#[derive(Debug, Clone)]
pub struct DirtyStatus {
    /// Whether there are any uncommitted changes.
    pub is_dirty: bool,

    /// Number of modified files.
    pub modified_count: usize,

    /// Number of untracked files.
    pub untracked_count: usize,

    /// Summary of changes (first few files).
    pub summary: String,
}

impl DirtyStatus {
    /// Check if a worktree has uncommitted changes.
    pub fn check<P: AsRef<Path>>(worktree_path: P) -> Result<Self, RemoveRoomError> {
        let path = worktree_path.as_ref();

        if !path.exists() {
            // Path doesn't exist, consider it not dirty (orphaned)
            return Ok(Self {
                is_dirty: false,
                modified_count: 0,
                untracked_count: 0,
                summary: String::new(),
            });
        }

        let result = GitCommand::new("status")
            .args(&["--porcelain"])
            .current_dir(path)
            .run()
            .map_err(|e| RemoveRoomError::StatusCheck(e.to_string()))?;

        if !result.success() {
            return Err(RemoveRoomError::StatusCheck(result.stderr));
        }

        let lines: Vec<&str> = result.stdout.lines().collect();
        let modified_count = lines.iter().filter(|l| !l.starts_with("??")).count();
        let untracked_count = lines.iter().filter(|l| l.starts_with("??")).count();
        let is_dirty = !lines.is_empty();

        // Build summary (first 5 files)
        let summary = lines
            .iter()
            .take(5)
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(Self {
            is_dirty,
            modified_count,
            untracked_count,
            summary,
        })
    }
}

/// Remove a room's worktree.
///
/// This removes the git worktree but does NOT delete the branch.
pub fn remove_worktree<P: AsRef<Path>>(worktree_path: P) -> Result<(), RemoveRoomError> {
    let path_str = worktree_path.as_ref().to_string_lossy().to_string();

    let result = GitCommand::new("worktree")
        .args(&["remove", &path_str])
        .run()
        .map_err(|e| RemoveRoomError::WorktreeRemoval(e.to_string()))?;

    if !result.success() {
        // If the worktree doesn't exist, try force removal
        if result.stderr.contains("is not a working tree") {
            return Ok(());
        }
        return Err(RemoveRoomError::WorktreeRemoval(result.stderr));
    }

    Ok(())
}

/// Force remove a room's worktree (even with uncommitted changes).
pub fn remove_worktree_force<P: AsRef<Path>>(worktree_path: P) -> Result<(), RemoveRoomError> {
    let path_str = worktree_path.as_ref().to_string_lossy().to_string();

    let result = GitCommand::new("worktree")
        .args(&["remove", "--force", &path_str])
        .run()
        .map_err(|e| RemoveRoomError::WorktreeRemoval(e.to_string()))?;

    if !result.success() {
        return Err(RemoveRoomError::WorktreeRemoval(result.stderr));
    }

    Ok(())
}

/// Remove a room by name.
///
/// Returns the removed room's name on success.
pub fn remove_room(
    state: &mut RoomsState,
    room_name: &str,
    force: bool,
) -> Result<String, RemoveRoomError> {
    // Find the room
    let room = state
        .find_by_name(room_name)
        .ok_or_else(|| RemoveRoomError::NotFound(room_name.to_string()))?;

    let path = room.path.clone();
    let name = room.name.clone();

    // Remove the worktree
    if force {
        remove_worktree_force(&path)?;
    } else {
        remove_worktree(&path)?;
    }

    // Remove from state
    state.remove_by_name(room_name);

    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    fn setup_test_repo() -> (tempfile::TempDir, std::path::PathBuf) {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_path_buf();

        Command::new("git")
            .args(["init"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        (temp_dir, repo_path)
    }

    #[test]
    fn test_dirty_status_clean() {
        let (_temp_dir, repo_path) = setup_test_repo();

        let status = DirtyStatus::check(&repo_path).unwrap();
        assert!(!status.is_dirty);
        assert_eq!(status.modified_count, 0);
        assert_eq!(status.untracked_count, 0);
    }

    #[test]
    fn test_dirty_status_untracked() {
        let (_temp_dir, repo_path) = setup_test_repo();

        // Create an untracked file
        fs::write(repo_path.join("untracked.txt"), "test").unwrap();

        let status = DirtyStatus::check(&repo_path).unwrap();
        assert!(status.is_dirty);
        assert_eq!(status.modified_count, 0);
        assert_eq!(status.untracked_count, 1);
    }

    #[test]
    fn test_dirty_status_modified() {
        let (_temp_dir, repo_path) = setup_test_repo();

        // Create and commit a file
        fs::write(repo_path.join("test.txt"), "original").unwrap();
        Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add test"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Modify the file
        fs::write(repo_path.join("test.txt"), "modified").unwrap();

        let status = DirtyStatus::check(&repo_path).unwrap();
        assert!(status.is_dirty);
        assert_eq!(status.modified_count, 1);
    }

    #[test]
    fn test_dirty_status_nonexistent_path() {
        let status = DirtyStatus::check("/nonexistent/path").unwrap();
        assert!(!status.is_dirty);
    }
}
