#![allow(dead_code)]

use std::path::Path;

use thiserror::Error;

use crate::git::command::GitCommand;
use crate::room::naming::validate_room_name;
use crate::state::RoomsState;

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
/// - The room's display name in state
/// - The worktree directory (via `git worktree move`)
///
/// The git branch name remains unchanged.
pub fn rename_room(
    repo_root: &Path,
    rooms_dir: &Path,
    state: &mut RoomsState,
    current_name: &str,
    new_name: &str,
) -> Result<String, RenameRoomError> {
    // Validate new name
    validate_room_name(new_name).map_err(RenameRoomError::InvalidName)?;

    // Check if same name
    if current_name == new_name {
        return Err(RenameRoomError::SameName);
    }

    // Check if new name already exists in state
    if state.name_exists(new_name) {
        return Err(RenameRoomError::NameExists(new_name.to_string()));
    }

    // Find the room to get its current path
    let room = state
        .find_by_name(current_name)
        .ok_or_else(|| RenameRoomError::NotFound(current_name.to_string()))?;

    let old_path = room.path.clone();
    let new_path = rooms_dir.join(new_name);

    // Check if destination path already exists on filesystem
    if new_path.exists() {
        return Err(RenameRoomError::PathExists(
            new_path.to_string_lossy().to_string(),
        ));
    }

    // Move the worktree using git (must be run from repo root)
    let result = GitCommand::new("worktree")
        .args(&["move", &old_path.to_string_lossy(), &new_path.to_string_lossy()])
        .current_dir(repo_root)
        .run()
        .map_err(|e| RenameRoomError::WorktreeMove(e.to_string()))?;

    if !result.success() {
        return Err(RenameRoomError::WorktreeMove(result.stderr));
    }

    // Update the room in state
    let room = state
        .find_by_name_mut(current_name)
        .ok_or_else(|| RenameRoomError::NotFound(current_name.to_string()))?;

    let old_name = room.name.clone();
    room.name = new_name.to_string();
    room.path = new_path;

    Ok(old_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Room, RoomStatus};
    use std::path::PathBuf;
    use std::process::Command;
    use uuid::Uuid;

    fn create_test_room(name: &str, path: PathBuf) -> Room {
        Room {
            id: Uuid::new_v4(),
            name: name.to_string(),
            branch: name.to_string(),
            path,
            created_at: chrono::Utc::now(),
            last_used_at: chrono::Utc::now(),
            status: RoomStatus::Ready,
            last_error: None,
        }
    }

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
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path();
        let rooms_dir = temp_dir.path();
        let mut state = RoomsState::default();

        let result = rename_room(repo_root, rooms_dir, &mut state, "nonexistent", "new-name");
        assert!(matches!(result, Err(RenameRoomError::NotFound(_))));
    }

    #[test]
    fn test_rename_room_invalid_name() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path();
        let rooms_dir = temp_dir.path();
        let mut state = RoomsState::default();
        state
            .rooms
            .push(create_test_room("old-name", rooms_dir.join("old-name")));

        let result = rename_room(repo_root, rooms_dir, &mut state, "old-name", "Invalid Name");
        assert!(matches!(result, Err(RenameRoomError::InvalidName(_))));
    }

    #[test]
    fn test_rename_room_name_exists() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path();
        let rooms_dir = temp_dir.path();
        let mut state = RoomsState::default();
        state
            .rooms
            .push(create_test_room("room-a", rooms_dir.join("room-a")));
        state
            .rooms
            .push(create_test_room("room-b", rooms_dir.join("room-b")));

        let result = rename_room(repo_root, rooms_dir, &mut state, "room-a", "room-b");
        assert!(matches!(result, Err(RenameRoomError::NameExists(_))));
    }

    #[test]
    fn test_rename_room_same_name() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path();
        let rooms_dir = temp_dir.path();
        let mut state = RoomsState::default();
        state
            .rooms
            .push(create_test_room("my-room", rooms_dir.join("my-room")));

        let result = rename_room(repo_root, rooms_dir, &mut state, "my-room", "my-room");
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
            .args(["worktree", "add", "-b", "old-name", &old_path.to_string_lossy()])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        let mut state = RoomsState::default();
        state.rooms.push(create_test_room("old-name", old_path.clone()));

        // Rename the room
        let result = rename_room(&repo_path, &rooms_dir, &mut state, "old-name", "new-name");
        assert!(result.is_ok(), "rename failed: {:?}", result.err());
        assert_eq!(result.unwrap(), "old-name");

        // Verify state was updated
        let room = state.find_by_name("new-name");
        assert!(room.is_some());
        let room = room.unwrap();
        assert_eq!(room.name, "new-name");
        assert_eq!(room.path, rooms_dir.join("new-name"));

        // Verify old name no longer exists
        assert!(state.find_by_name("old-name").is_none());

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
            .args(["worktree", "add", "-b", "old-name", &old_path.to_string_lossy()])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Create a directory at the destination path
        let new_path = rooms_dir.join("new-name");
        std::fs::create_dir_all(&new_path).unwrap();

        let mut state = RoomsState::default();
        state.rooms.push(create_test_room("old-name", old_path));

        let result = rename_room(&repo_path, &rooms_dir, &mut state, "old-name", "new-name");
        assert!(matches!(result, Err(RenameRoomError::PathExists(_))));
    }
}
