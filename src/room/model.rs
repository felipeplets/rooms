//! Room model types for representing rooms derived from git worktrees.

// Allow dead code for now - these types will be used in later implementation steps
#![allow(dead_code)]

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::git::Worktree;

/// Room status in the lifecycle state machine.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RoomStatus {
    /// Room exists, no active background operation.
    #[default]
    Idle,

    /// Creating worktree/branch.
    Creating,

    /// Running post-create commands.
    PostCreateRunning,

    /// Terminal ready, no active background ops.
    Ready,

    /// Last operation failed.
    Error,

    /// Removing room (worktree removal).
    Deleting,

    /// Worktree missing on disk / inconsistent state.
    Orphaned,
}

/// Room information derived from a git worktree.
///
/// This struct represents a room as discovered from `git worktree list`.
/// Unlike the persisted `Room` struct, this is a lightweight view that
/// derives its identity from the worktree directory name.
#[derive(Debug, Clone)]
pub struct RoomInfo {
    /// Room name (derived from directory name).
    pub name: String,

    /// Git branch name, if any (None if detached HEAD).
    pub branch: Option<String>,

    /// Path to the worktree directory.
    pub path: PathBuf,

    /// Current status in the lifecycle.
    pub status: RoomStatus,

    /// Whether this worktree is marked as prunable by git.
    pub is_prunable: bool,

    /// Last error message if status is Error.
    pub last_error: Option<String>,

    /// Whether this worktree is the primary worktree.
    pub is_primary: bool,
}

impl RoomInfo {
    /// Set the room status to Error with a message.
    pub fn set_error(&mut self, message: String) {
        self.status = RoomStatus::Error;
        self.last_error = Some(message);
    }

    /// Clear any error and set status to Ready.
    pub fn set_ready(&mut self) {
        self.status = RoomStatus::Ready;
        self.last_error = None;
    }
}

impl From<&Worktree> for RoomInfo {
    fn from(worktree: &Worktree) -> Self {
        let name = worktree.name().unwrap_or("unknown").to_string();

        let status = if worktree.is_prunable() {
            RoomStatus::Orphaned
        } else {
            RoomStatus::Ready
        };

        Self {
            name,
            branch: worktree.branch.clone(),
            path: worktree.path.clone(),
            status,
            is_prunable: worktree.is_prunable(),
            last_error: None,
            is_primary: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_info_from_worktree() {
        let worktree = Worktree {
            path: PathBuf::from("/home/user/repo/.rooms/quick-fox-a1b2"),
            head: "abc123".to_string(),
            branch: Some("quick-fox-a1b2".to_string()),
            is_main: false,
            prunable: None,
            locked: None,
        };

        let room_info = RoomInfo::from(&worktree);

        assert_eq!(room_info.name, "quick-fox-a1b2");
        assert_eq!(room_info.branch, Some("quick-fox-a1b2".to_string()));
        assert_eq!(
            room_info.path,
            PathBuf::from("/home/user/repo/.rooms/quick-fox-a1b2")
        );
        assert_eq!(room_info.status, RoomStatus::Ready);
        assert!(!room_info.is_prunable);
        assert!(room_info.last_error.is_none());
        assert!(!room_info.is_primary);
    }

    #[test]
    fn test_room_info_from_prunable_worktree() {
        let worktree = Worktree {
            path: PathBuf::from("/home/user/repo/.rooms/orphaned-room"),
            head: "abc123".to_string(),
            branch: Some("orphaned-room".to_string()),
            is_main: false,
            prunable: Some("gitdir file points to non-existent location".to_string()),
            locked: None,
        };

        let room_info = RoomInfo::from(&worktree);

        assert_eq!(room_info.name, "orphaned-room");
        assert_eq!(room_info.status, RoomStatus::Orphaned);
        assert!(room_info.is_prunable);
    }

    #[test]
    fn test_room_info_from_detached_worktree() {
        let worktree = Worktree {
            path: PathBuf::from("/home/user/repo/.rooms/detached-wt"),
            head: "abc123".to_string(),
            branch: None,
            is_main: false,
            prunable: None,
            locked: None,
        };

        let room_info = RoomInfo::from(&worktree);

        assert_eq!(room_info.name, "detached-wt");
        assert_eq!(room_info.branch, None);
        assert_eq!(room_info.status, RoomStatus::Ready);
        assert!(!room_info.is_primary);
    }

    #[test]
    fn test_room_info_set_error() {
        let mut room_info = RoomInfo {
            name: "test".to_string(),
            branch: Some("test".to_string()),
            path: PathBuf::from("/test"),
            status: RoomStatus::Ready,
            is_prunable: false,
            last_error: None,
            is_primary: false,
        };

        room_info.set_error("something went wrong".to_string());

        assert_eq!(room_info.status, RoomStatus::Error);
        assert_eq!(
            room_info.last_error,
            Some("something went wrong".to_string())
        );
    }

    #[test]
    fn test_room_info_set_ready() {
        let mut room_info = RoomInfo {
            name: "test".to_string(),
            branch: Some("test".to_string()),
            path: PathBuf::from("/test"),
            status: RoomStatus::Error,
            is_prunable: false,
            last_error: Some("previous error".to_string()),
            is_primary: false,
        };

        room_info.set_ready();

        assert_eq!(room_info.status, RoomStatus::Ready);
        assert!(room_info.last_error.is_none());
    }

    #[test]
    fn test_room_status_serialization() {
        assert_eq!(
            serde_json::to_string(&RoomStatus::Creating).unwrap(),
            "\"creating\""
        );
        assert_eq!(
            serde_json::to_string(&RoomStatus::PostCreateRunning).unwrap(),
            "\"post_create_running\""
        );
        assert_eq!(
            serde_json::to_string(&RoomStatus::Orphaned).unwrap(),
            "\"orphaned\""
        );
    }
}
