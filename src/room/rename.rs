#![allow(dead_code)]

use std::path::Path;

use thiserror::Error;

use crate::git::command::GitCommand;
use crate::git::list_worktrees_from;
use crate::room::discovery::is_worktree_in_rooms_dir;
use crate::room::naming::validate_room_name;

#[derive(Error, Debug)]
pub enum RenameRoomError {
    #[error("room '{0}' not found")]
    NotFound(String),

    #[error("invalid name: {0}")]
    InvalidName(&'static str),

    #[error("a room named '{0}' already exists")]
    NameExists(String),

    #[error("new name is the same as current name")]
    SameName,

    #[error("destination path already exists: {0}")]
    PathExists(String),

    #[error("failed to move worktree: {0}")]
    WorktreeMove(String),
}

/// Rename a room.
///
/// This changes:
/// - The worktree directory (via `git worktree move`)
///
/// The git branch name remains unchanged.
pub fn rename_room(
    repo_root: &Path,
    rooms_dir: &Path,
    current_name: &str,
    new_name: &str,
) -> Result<String, RenameRoomError> {
    // Validate new name
    validate_room_name(new_name).map_err(RenameRoomError::InvalidName)?;

    // Check if same name
    if current_name == new_name {
        return Err(RenameRoomError::SameName);
    }

    let worktrees = list_worktrees_from(repo_root)
        .map_err(|_| RenameRoomError::NotFound(current_name.to_string()))?;
    let rooms_dir_canonical = rooms_dir
        .canonicalize()
        .unwrap_or_else(|_| rooms_dir.to_path_buf());

    let existing_names: Vec<String> = worktrees
        .iter()
        .filter(|worktree| is_worktree_in_rooms_dir(worktree, &rooms_dir_canonical))
        .filter_map(|worktree| worktree.name().map(|name| name.to_string()))
        .collect();

    // Check if new name already exists
    if existing_names.iter().any(|name| name == new_name) {
        return Err(RenameRoomError::NameExists(new_name.to_string()));
    }

    // Find the room to get its current path
    let old_path = worktrees
        .iter()
        .find(|worktree| {
            is_worktree_in_rooms_dir(worktree, &rooms_dir_canonical)
                && worktree.name() == Some(current_name)
        })
        .map(|worktree| worktree.path.clone())
        .ok_or_else(|| RenameRoomError::NotFound(current_name.to_string()))?;
    let new_path = rooms_dir.join(new_name);

    // Check if destination path already exists on filesystem
    if new_path.exists() {
        return Err(RenameRoomError::PathExists(
            new_path.to_string_lossy().to_string(),
        ));
    }

    // Move the worktree using git (must be run from repo root)
    let result = GitCommand::new("worktree")
        .args(&[
            "move",
            &old_path.to_string_lossy(),
            &new_path.to_string_lossy(),
        ])
        .current_dir(repo_root)
        .run()
        .map_err(|e| RenameRoomError::WorktreeMove(e.to_string()))?;

    if !result.success() {
        return Err(RenameRoomError::WorktreeMove(result.stderr));
    }

    Ok(current_name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;

    fn setup_test_repo() -> (tempfile::TempDir, PathBuf) {
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
    fn test_rename_room_not_found() {
        let (_temp_dir, repo_root) = setup_test_repo();
        let rooms_dir = repo_root.join(".rooms");
        std::fs::create_dir_all(&rooms_dir).unwrap();

        let result = rename_room(repo_root, rooms_dir, "nonexistent", "new-name");
        assert!(matches!(result, Err(RenameRoomError::NotFound(_))));
    }

    #[test]
    fn test_rename_room_invalid_name() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path();
        let rooms_dir = temp_dir.path();

        let result = rename_room(repo_root, rooms_dir, "old-name", "Invalid Name");
        assert!(matches!(result, Err(RenameRoomError::InvalidName(_))));
    }

    #[test]
    fn test_rename_room_name_exists() {
        let (_temp_dir, repo_root) = setup_test_repo();
        let rooms_dir = repo_root.join(".rooms");
        std::fs::create_dir_all(&rooms_dir).unwrap();

        let existing_path = rooms_dir.join("room-b");
        Command::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                "room-b",
                &existing_path.to_string_lossy(),
            ])
            .current_dir(&repo_root)
            .output()
            .unwrap();

        let result = rename_room(repo_root, rooms_dir, "room-a", "room-b");
        assert!(matches!(result, Err(RenameRoomError::NameExists(_))));
    }

    #[test]
    fn test_rename_room_same_name() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path();
        let rooms_dir = temp_dir.path();

        let result = rename_room(repo_root, rooms_dir, "my-room", "my-room");
        assert!(matches!(result, Err(RenameRoomError::SameName)));
    }

    #[test]
    fn test_rename_room_with_worktree() {
        let (_temp_dir, repo_path) = setup_test_repo();
        let rooms_dir = repo_path.join(".rooms");
        std::fs::create_dir_all(&rooms_dir).unwrap();

        // Create a worktree
        let old_path = rooms_dir.join("old-name");
        Command::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                "old-name",
                &old_path.to_string_lossy(),
            ])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Rename the room
        let result = rename_room(&repo_path, &rooms_dir, "old-name", "new-name");
        assert!(result.is_ok(), "rename failed: {:?}", result.err());
        assert_eq!(result.unwrap(), "old-name");

        // Verify filesystem was updated
        assert!(!old_path.exists());
        assert!(rooms_dir.join("new-name").exists());
    }

    #[test]
    fn test_rename_room_path_exists() {
        let (_temp_dir, repo_path) = setup_test_repo();
        let rooms_dir = repo_path.join(".rooms");
        std::fs::create_dir_all(&rooms_dir).unwrap();

        // Create a worktree
        let old_path = rooms_dir.join("old-name");
        Command::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                "old-name",
                &old_path.to_string_lossy(),
            ])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Create a directory at the destination path
        let new_path = rooms_dir.join("new-name");
        std::fs::create_dir_all(&new_path).unwrap();

        let result = rename_room(&repo_path, &rooms_dir, "old-name", "new-name");
        assert!(matches!(result, Err(RenameRoomError::PathExists(_))));
    }
}
