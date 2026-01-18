//! Room discovery from git worktrees.
//!
//! This module provides functions to discover rooms by querying git worktrees
//! and merging the results with transient state.

// Allow dead code for now - these utilities will be used in later implementation steps
#![allow(dead_code)]

use std::path::Path;

use thiserror::Error;

use crate::git::command::CommandError;
use crate::git::{list_worktrees_from, Worktree};
use crate::room::{RoomInfo, RoomStatus};
use crate::state::TransientStateStore;

/// Error type for room discovery operations.
#[derive(Error, Debug)]
pub enum DiscoveryError {
    /// Failed to list git worktrees.
    #[error("failed to list worktrees: {0}")]
    WorktreeList(#[from] CommandError),

    /// Rooms directory does not exist or is inaccessible.
    #[error("rooms directory does not exist or is inaccessible: {path}")]
    InvalidRoomsDir { path: String },
}

/// Discover rooms by listing git worktrees and filtering to the rooms directory.
///
/// This function:
/// 1. Lists all worktrees in the repository
/// 2. Filters to only worktrees inside the rooms directory
/// 3. Excludes the main worktree
/// 4. Merges transient state (Creating, Deleting, Error) with discovered rooms
/// 5. Marks prunable worktrees as Orphaned
///
/// # Arguments
/// * `repo_root` - Path to the repository root. Must be a valid git repository,
///   otherwise this function will return a `WorktreeList` error from the
///   underlying git command.
/// * `rooms_dir` - Path to the rooms directory (e.g., `.rooms/`)
/// * `transient` - Transient state store for in-memory status
///
/// # Returns
/// A vector of `RoomInfo` representing discovered rooms.
///
/// # Errors
/// * `DiscoveryError::InvalidRoomsDir` - If `rooms_dir` does not exist or is not a directory
/// * `DiscoveryError::WorktreeList` - If `repo_root` is not a valid git repository or
///   the git command fails
pub fn discover_rooms(
    repo_root: &Path,
    rooms_dir: &Path,
    transient: &TransientStateStore,
) -> Result<Vec<RoomInfo>, DiscoveryError> {
    // Validate that rooms_dir exists and is accessible
    if !rooms_dir.exists() || !rooms_dir.is_dir() {
        return Err(DiscoveryError::InvalidRoomsDir {
            path: rooms_dir.to_string_lossy().to_string(),
        });
    }

    // List all worktrees from the repository root
    let worktrees = list_worktrees_from(repo_root)?;

    // Canonicalize rooms_dir for reliable path comparison
    let rooms_dir_canonical = rooms_dir
        .canonicalize()
        .unwrap_or_else(|_| rooms_dir.to_path_buf());

    // Filter to worktrees inside rooms_dir and convert to RoomInfo
    let rooms: Vec<RoomInfo> = worktrees
        .iter()
        .filter(|wt| {
            // Skip the main worktree
            if wt.is_main {
                return false;
            }

            // Check if worktree is inside rooms_dir using canonicalized paths
            is_worktree_in_rooms_dir(wt, &rooms_dir_canonical)
        })
        .map(|wt| {
            let mut room_info = RoomInfo::from(wt);

            // Apply transient state if present
            if let Some(transient_state) = transient.get(&room_info.name) {
                room_info.status = transient_state.status.clone();
                room_info.last_error = transient_state.last_error.clone();
            }

            room_info
        })
        .collect();

    Ok(rooms)
}

/// Check if a worktree is located inside the rooms directory.
///
/// Uses canonicalized paths for reliable comparison, with a fallback to
/// starts_with comparison if canonicalization fails.
fn is_worktree_in_rooms_dir(worktree: &Worktree, rooms_dir_canonical: &Path) -> bool {
    // Try to canonicalize the worktree path
    if let Ok(wt_canonical) = worktree.path.canonicalize() {
        return wt_canonical.starts_with(rooms_dir_canonical);
    }

    // Fallback when canonicalization fails (e.g., non-existent path, permissions,
    // or symlink resolution issues). Use the original paths but normalize them by
    // stripping trailing slashes for a best-effort comparison.
    // For non-existent or inaccessible paths, compare normalized string representations.
    let wt_str = normalize_path_string(&worktree.path);
    let rooms_str = normalize_path_string(rooms_dir_canonical);
    wt_str.starts_with(&rooms_str)
}

/// Normalize a path to a string for comparison.
///
/// This handles:
/// - Trailing slashes (both Unix `/` and Windows `\`)
/// - Redundant path separators (e.g., `/path//to/dir` -> `/path/to/dir`)
fn normalize_path_string(path: &Path) -> String {
    let path_str = path.to_string_lossy();

    // Replace redundant separators and normalize to forward slashes for comparison
    let normalized: String = path_str
        .chars()
        .fold((String::new(), false), |(mut acc, was_sep), c| {
            let is_sep = c == '/' || c == '\\';
            if is_sep {
                if !was_sep {
                    acc.push('/');
                }
                (acc, true)
            } else {
                acc.push(c);
                (acc, false)
            }
        })
        .0;

    // Remove trailing slashes
    normalized.trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    /// Helper struct for test git repositories.
    /// The TempDir is kept alive to prevent cleanup until the test completes.
    struct TestRepo {
        _temp_dir: tempfile::TempDir,
        path: PathBuf,
    }

    impl TestRepo {
        /// Create a new temporary git repository with an initial commit.
        fn new() -> Self {
            let temp_dir = tempfile::tempdir().unwrap();
            let path = temp_dir.path().to_path_buf();

            Command::new("git")
                .args(["init"])
                .current_dir(&path)
                .output()
                .unwrap();

            Command::new("git")
                .args(["config", "user.email", "test@test.com"])
                .current_dir(&path)
                .output()
                .unwrap();

            Command::new("git")
                .args(["config", "user.name", "Test"])
                .current_dir(&path)
                .output()
                .unwrap();

            Command::new("git")
                .args(["commit", "--allow-empty", "-m", "init"])
                .current_dir(&path)
                .output()
                .unwrap();

            Self {
                _temp_dir: temp_dir,
                path,
            }
        }

        /// Get the repository path.
        fn path(&self) -> &Path {
            &self.path
        }

        /// Create a rooms directory and return its path.
        fn create_rooms_dir(&self) -> PathBuf {
            let rooms_dir = self.path.join(".rooms");
            std::fs::create_dir(&rooms_dir).unwrap();
            rooms_dir
        }

        /// Add a worktree with the given name in the specified directory.
        fn add_worktree(&self, parent_dir: &Path, name: &str) -> PathBuf {
            let worktree_path = parent_dir.join(name);
            Command::new("git")
                .args([
                    "worktree",
                    "add",
                    "-b",
                    name,
                    worktree_path.to_str().unwrap(),
                ])
                .current_dir(&self.path)
                .output()
                .unwrap();
            worktree_path
        }
    }

    #[test]
    fn test_normalize_path_string() {
        // Basic trailing slash removal
        assert_eq!(
            normalize_path_string(&PathBuf::from("/home/user/repo/.rooms/")),
            "/home/user/repo/.rooms"
        );
        assert_eq!(
            normalize_path_string(&PathBuf::from("/home/user/repo/.rooms")),
            "/home/user/repo/.rooms"
        );

        // Multiple trailing slashes
        assert_eq!(
            normalize_path_string(&PathBuf::from("/home/user/repo/.rooms///")),
            "/home/user/repo/.rooms"
        );

        // Redundant separators in the middle
        assert_eq!(
            normalize_path_string(&PathBuf::from("/home//user///repo/.rooms")),
            "/home/user/repo/.rooms"
        );

        // Windows-style backslashes (normalized to forward slashes)
        assert_eq!(
            normalize_path_string(&PathBuf::from("C:\\Users\\test\\repo\\.rooms\\")),
            "C:/Users/test/repo/.rooms"
        );

        // Mixed separators
        assert_eq!(
            normalize_path_string(&PathBuf::from("/home\\user//repo\\.rooms/")),
            "/home/user/repo/.rooms"
        );
    }

    #[test]
    fn test_is_worktree_in_rooms_dir_prunable() {
        // Test prunable worktree detection with non-existent paths
        let worktree = Worktree {
            path: PathBuf::from("/home/user/repo/.rooms/orphaned-room"),
            head: "abc123".to_string(),
            branch: Some("orphaned-room".to_string()),
            is_main: false,
            prunable: Some("gitdir file points to non-existent location".to_string()),
            locked: None,
        };

        let rooms_dir = PathBuf::from("/home/user/repo/.rooms");

        // This should match even though the path doesn't exist
        assert!(is_worktree_in_rooms_dir(&worktree, &rooms_dir));
    }

    #[test]
    fn test_is_worktree_in_rooms_dir_prunable_outside() {
        // Test prunable worktree that is NOT in rooms_dir
        let worktree = Worktree {
            path: PathBuf::from("/home/user/other-worktrees/feature"),
            head: "abc123".to_string(),
            branch: Some("feature".to_string()),
            is_main: false,
            prunable: Some("gitdir file points to non-existent location".to_string()),
            locked: None,
        };

        let rooms_dir = PathBuf::from("/home/user/repo/.rooms");

        assert!(!is_worktree_in_rooms_dir(&worktree, &rooms_dir));
    }

    #[test]
    fn test_discovery_error_invalid_rooms_dir() {
        // Test that discovery fails with InvalidRoomsDir for non-existent directory
        let repo_root = PathBuf::from("/some/repo");
        let rooms_dir = PathBuf::from("/this/path/definitely/does/not/exist/rooms");
        let transient = TransientStateStore::new();

        let result = discover_rooms(&repo_root, &rooms_dir, &transient);

        assert!(result.is_err());
        match result.unwrap_err() {
            DiscoveryError::InvalidRoomsDir { path } => {
                assert!(path.contains("does/not/exist"));
            }
            other => panic!("Expected InvalidRoomsDir error, got: {:?}", other),
        }
    }

    #[test]
    fn test_discover_rooms_empty() {
        let repo = TestRepo::new();
        let rooms_dir = repo.create_rooms_dir();

        let transient = TransientStateStore::new();
        let result = discover_rooms(repo.path(), &rooms_dir, &transient);

        assert!(result.is_ok());
        let rooms = result.unwrap();
        assert!(rooms.is_empty());
    }

    #[test]
    fn test_discover_rooms_with_worktree() {
        let repo = TestRepo::new();
        let rooms_dir = repo.create_rooms_dir();
        repo.add_worktree(&rooms_dir, "test-room");

        let transient = TransientStateStore::new();
        let result = discover_rooms(repo.path(), &rooms_dir, &transient);

        assert!(result.is_ok());
        let rooms = result.unwrap();
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].name, "test-room");
        assert_eq!(rooms[0].branch, Some("test-room".to_string()));
        assert_eq!(rooms[0].status, RoomStatus::Ready);
    }

    #[test]
    fn test_discover_rooms_applies_transient_state_creating() {
        let repo = TestRepo::new();
        let rooms_dir = repo.create_rooms_dir();
        repo.add_worktree(&rooms_dir, "creating-room");

        // Set transient state
        let mut transient = TransientStateStore::new();
        transient.set_status("creating-room", RoomStatus::Creating);

        let result = discover_rooms(repo.path(), &rooms_dir, &transient);

        assert!(result.is_ok());
        let rooms = result.unwrap();
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].name, "creating-room");
        // Transient status should override the default Ready status
        assert_eq!(rooms[0].status, RoomStatus::Creating);
    }

    #[test]
    fn test_discover_rooms_applies_transient_state_error() {
        let repo = TestRepo::new();
        let rooms_dir = repo.create_rooms_dir();
        repo.add_worktree(&rooms_dir, "error-room");

        // Set transient error state with message
        let mut transient = TransientStateStore::new();
        transient.set_error("error-room", "Post-create command failed".to_string());

        let result = discover_rooms(repo.path(), &rooms_dir, &transient);

        assert!(result.is_ok());
        let rooms = result.unwrap();
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].name, "error-room");
        assert_eq!(rooms[0].status, RoomStatus::Error);
        assert_eq!(
            rooms[0].last_error,
            Some("Post-create command failed".to_string())
        );
    }

    #[test]
    fn test_discover_rooms_applies_transient_state_deleting() {
        let repo = TestRepo::new();
        let rooms_dir = repo.create_rooms_dir();
        repo.add_worktree(&rooms_dir, "deleting-room");

        let mut transient = TransientStateStore::new();
        transient.set_status("deleting-room", RoomStatus::Deleting);

        let result = discover_rooms(repo.path(), &rooms_dir, &transient);

        assert!(result.is_ok());
        let rooms = result.unwrap();
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].name, "deleting-room");
        assert_eq!(rooms[0].status, RoomStatus::Deleting);
    }

    #[test]
    fn test_discover_rooms_applies_transient_state_post_create_running() {
        let repo = TestRepo::new();
        let rooms_dir = repo.create_rooms_dir();
        repo.add_worktree(&rooms_dir, "post-create-room");

        let mut transient = TransientStateStore::new();
        transient.set_status("post-create-room", RoomStatus::PostCreateRunning);

        let result = discover_rooms(repo.path(), &rooms_dir, &transient);

        assert!(result.is_ok());
        let rooms = result.unwrap();
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].name, "post-create-room");
        assert_eq!(rooms[0].status, RoomStatus::PostCreateRunning);
    }

    #[test]
    fn test_discover_rooms_with_prunable_worktree() {
        let repo = TestRepo::new();
        let rooms_dir = repo.create_rooms_dir();
        let worktree_path = repo.add_worktree(&rooms_dir, "orphan-room");

        // Delete the worktree directory to make it prunable
        std::fs::remove_dir_all(&worktree_path).unwrap();

        let transient = TransientStateStore::new();
        let result = discover_rooms(repo.path(), &rooms_dir, &transient);

        assert!(result.is_ok());
        let rooms = result.unwrap();
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].name, "orphan-room");
        // Prunable worktrees should have Orphaned status
        assert_eq!(rooms[0].status, RoomStatus::Orphaned);
        assert!(rooms[0].is_prunable);
    }

    #[test]
    fn test_discover_rooms_excludes_worktrees_outside_rooms_dir() {
        let repo = TestRepo::new();
        let rooms_dir = repo.create_rooms_dir();

        // Add a worktree inside rooms dir
        repo.add_worktree(&rooms_dir, "inside-room");

        // Add a worktree outside rooms dir
        repo.add_worktree(repo.path(), "outside-worktree");

        let transient = TransientStateStore::new();
        let result = discover_rooms(repo.path(), &rooms_dir, &transient);

        assert!(result.is_ok());
        let rooms = result.unwrap();

        // Should only include the worktree inside rooms dir
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].name, "inside-room");
    }
}
