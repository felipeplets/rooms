#![allow(dead_code)]

use std::path::PathBuf;

use thiserror::Error;

use crate::git::command::{CommandError, GitCommand};
use crate::state::{Room, RoomStatus, RoomsState};

use super::naming::{generate_unique_room_name, sanitize_room_name, validate_room_name};

#[derive(Error, Debug)]
pub enum CreateRoomError {
    #[error("invalid room name: {0}")]
    InvalidName(&'static str),

    #[error("room '{0}' already exists")]
    NameExists(String),

    #[error("failed to create worktree: {0}")]
    WorktreeCreation(String),

    #[error("git command failed: {0}")]
    GitError(#[from] CommandError),

    #[error("failed to save state: {0}")]
    StateSave(String),
}

/// Options for creating a new room.
#[derive(Debug, Clone, Default)]
pub struct CreateRoomOptions {
    /// Room name (optional, will be generated if not provided).
    pub name: Option<String>,

    /// Branch name (optional, defaults to room name).
    pub branch: Option<String>,

    /// Base branch to create from (optional, defaults to HEAD).
    pub base_branch: Option<String>,
}

/// Create a new room with a git worktree.
///
/// Returns the created Room on success.
pub fn create_room(
    rooms_dir: &std::path::Path,
    state: &mut RoomsState,
    options: CreateRoomOptions,
) -> Result<Room, CreateRoomError> {
    create_room_in_repo(rooms_dir, state, options, None)
}

/// Create a new room with a git worktree, specifying the repository directory.
///
/// This is primarily for testing to avoid changing the current directory.
/// If repo_dir is None, git commands will use the current directory.
fn create_room_in_repo(
    rooms_dir: &std::path::Path,
    state: &mut RoomsState,
    options: CreateRoomOptions,
    repo_dir: Option<&std::path::Path>,
) -> Result<Room, CreateRoomError> {
    // Determine room name
    let name = match options.name {
        Some(n) => {
            let sanitized = sanitize_room_name(&n);
            validate_room_name(&sanitized).map_err(CreateRoomError::InvalidName)?;
            if state.name_exists(&sanitized) {
                return Err(CreateRoomError::NameExists(sanitized));
            }
            sanitized
        }
        None => generate_unique_room_name(|n| state.name_exists(n)),
    };

    // Determine branch name (default to room name)
    let branch = options
        .branch
        .map(|b| sanitize_room_name(&b))
        .unwrap_or_else(|| name.clone());

    // Determine worktree path
    let worktree_path = rooms_dir.join(&name);

    // Check if path already exists
    if worktree_path.exists() {
        return Err(CreateRoomError::WorktreeCreation(format!(
            "path already exists: {}",
            worktree_path.display()
        )));
    }

    // Create the worktree
    // First, check if the branch exists
    let branch_exists = check_branch_exists_in_repo(&branch, repo_dir)?;

    let worktree_path_str = worktree_path.to_string_lossy().to_string();

    let result = if branch_exists {
        // Use existing branch
        let mut cmd = GitCommand::new("worktree").args(&["add", &worktree_path_str, &branch]);
        if let Some(dir) = repo_dir {
            cmd = cmd.current_dir(dir);
        }
        cmd.run()
    } else {
        // Create new branch from base (or HEAD)
        match &options.base_branch {
            Some(base) => {
                let mut cmd = GitCommand::new("worktree").args(&[
                    "add",
                    "-b",
                    &branch,
                    &worktree_path_str,
                    base,
                ]);
                if let Some(dir) = repo_dir {
                    cmd = cmd.current_dir(dir);
                }
                cmd.run()
            }
            None => {
                let mut cmd =
                    GitCommand::new("worktree").args(&["add", "-b", &branch, &worktree_path_str]);
                if let Some(dir) = repo_dir {
                    cmd = cmd.current_dir(dir);
                }
                cmd.run()
            }
        }
    };

    match result {
        Ok(output) if output.success() => {
            // Create room record
            let mut room = Room::new(name, branch, worktree_path);
            room.status = RoomStatus::Ready;

            // Add to state
            state.add_room(room.clone());

            Ok(room)
        }
        Ok(output) => Err(CreateRoomError::WorktreeCreation(output.stderr)),
        Err(e) => Err(CreateRoomError::GitError(e)),
    }
}

/// Check if a branch exists in the repository.
fn check_branch_exists(branch: &str) -> Result<bool, CommandError> {
    check_branch_exists_in_repo(branch, None)
}

/// Check if a branch exists in the repository, optionally in a specific directory.
fn check_branch_exists_in_repo(
    branch: &str,
    repo_dir: Option<&std::path::Path>,
) -> Result<bool, CommandError> {
    let mut cmd = GitCommand::new("rev-parse").args(&[
        "--verify",
        "--quiet",
        &format!("refs/heads/{}", branch),
    ]);

    if let Some(dir) = repo_dir {
        cmd = cmd.current_dir(dir);
    }

    let result = cmd.run()?;

    Ok(result.success())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn setup_test_repo() -> (tempfile::TempDir, PathBuf) {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize git repo
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

        // Create initial commit
        Command::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        (temp_dir, repo_path)
    }

    #[test]
    fn test_create_room_silent() {
        let (_temp_dir, repo_path) = setup_test_repo();
        let rooms_dir = repo_path.join(".rooms");
        std::fs::create_dir_all(&rooms_dir).unwrap();

        let mut state = RoomsState::default();
        let options = CreateRoomOptions::default();

        let result = create_room_in_repo(&rooms_dir, &mut state, options, Some(&repo_path));
        assert!(result.is_ok(), "Failed to create room: {:?}", result.err());

        let room = result.unwrap();
        assert!(!room.name.is_empty());
        assert!(room.path.exists());
        assert_eq!(state.rooms.len(), 1);
    }

    #[test]
    fn test_create_room_with_name() {
        let (_temp_dir, repo_path) = setup_test_repo();
        let rooms_dir = repo_path.join(".rooms");
        std::fs::create_dir_all(&rooms_dir).unwrap();

        let mut state = RoomsState::default();
        let options = CreateRoomOptions {
            name: Some("my-feature".to_string()),
            ..Default::default()
        };

        let result = create_room_in_repo(&rooms_dir, &mut state, options, Some(&repo_path));
        assert!(result.is_ok());

        let room = result.unwrap();
        assert_eq!(room.name, "my-feature");
        assert_eq!(room.branch, "my-feature");
    }

    #[test]
    fn test_create_room_duplicate_name_fails() {
        let (_temp_dir, repo_path) = setup_test_repo();
        let rooms_dir = repo_path.join(".rooms");
        std::fs::create_dir_all(&rooms_dir).unwrap();

        let mut state = RoomsState::default();

        // Create first room
        let options1 = CreateRoomOptions {
            name: Some("duplicate".to_string()),
            ..Default::default()
        };
        create_room_in_repo(&rooms_dir, &mut state, options1, Some(&repo_path)).unwrap();

        // Try to create room with same name
        let options2 = CreateRoomOptions {
            name: Some("duplicate".to_string()),
            ..Default::default()
        };
        let result = create_room_in_repo(&rooms_dir, &mut state, options2, Some(&repo_path));

        assert!(matches!(result, Err(CreateRoomError::NameExists(_))));
    }
}
