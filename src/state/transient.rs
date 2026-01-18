//! Transient (in-memory only) state for rooms.
//!
//! This module provides storage for temporary room states that don't need
//! to be persisted to disk. Examples include:
//! - Creating: worktree creation in progress
//! - PostCreateRunning: post-create commands running
//! - Deleting: worktree removal in progress
//! - Error: operation failed (with error message)

use std::collections::HashMap;

use crate::room::RoomStatus;

/// Transient state for a single room.
///
/// This holds temporary status information that doesn't need to be persisted.
#[derive(Debug, Clone)]
pub struct TransientRoomState {
    /// Current transient status.
    pub status: RoomStatus,

    /// Error message if status is Error.
    pub last_error: Option<String>,
}

impl TransientRoomState {
    /// Create a new transient state with the given status.
    pub fn new(status: RoomStatus) -> Self {
        Self {
            status,
            last_error: None,
        }
    }

    /// Create a new transient state with Error status and a message.
    pub fn with_error(message: String) -> Self {
        Self {
            status: RoomStatus::Error,
            last_error: Some(message),
        }
    }
}

/// In-memory store for transient room states.
///
/// Transient states are keyed by room name (directory name) and are not
/// persisted to disk. They are used to track temporary states like:
/// - Creating, PostCreateRunning, Deleting (operation in progress)
/// - Error (operation failed)
///
/// When a room's transient state is cleared, the room's status will be
/// determined by the git worktree state (Ready or Orphaned).
#[derive(Debug, Default)]
pub struct TransientStateStore {
    states: HashMap<String, TransientRoomState>,
}

impl TransientStateStore {
    /// Create a new empty transient state store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the transient status for a room.
    pub fn set_status(&mut self, room_name: &str, status: RoomStatus) {
        self.states
            .insert(room_name.to_string(), TransientRoomState::new(status));
    }

    /// Set an error status with a message for a room.
    pub fn set_error(&mut self, room_name: &str, message: String) {
        self.states.insert(
            room_name.to_string(),
            TransientRoomState::with_error(message),
        );
    }

    /// Get the transient state for a room, if any.
    pub fn get(&self, room_name: &str) -> Option<&TransientRoomState> {
        self.states.get(room_name)
    }

    /// Get the transient status for a room, if any.
    pub fn get_status(&self, room_name: &str) -> Option<&RoomStatus> {
        self.states.get(room_name).map(|s| &s.status)
    }

    /// Get the error message for a room, if any.
    pub fn get_error(&self, room_name: &str) -> Option<&str> {
        self.states
            .get(room_name)
            .and_then(|s| s.last_error.as_deref())
    }

    /// Check if a room has a transient state.
    pub fn has(&self, room_name: &str) -> bool {
        self.states.contains_key(room_name)
    }

    /// Remove the transient state for a room.
    ///
    /// Returns the removed state if it existed.
    pub fn remove(&mut self, room_name: &str) -> Option<TransientRoomState> {
        self.states.remove(room_name)
    }

    /// Clear all transient states.
    pub fn clear(&mut self) {
        self.states.clear();
    }

    /// Get the number of rooms with transient states.
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Check if there are no transient states.
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    /// Iterate over all room names with transient states.
    pub fn room_names(&self) -> impl Iterator<Item = &str> {
        self.states.keys().map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transient_room_state_new() {
        let state = TransientRoomState::new(RoomStatus::Creating);
        assert_eq!(state.status, RoomStatus::Creating);
        assert!(state.last_error.is_none());
    }

    #[test]
    fn test_transient_room_state_with_error() {
        let state = TransientRoomState::with_error("something failed".to_string());
        assert_eq!(state.status, RoomStatus::Error);
        assert_eq!(state.last_error, Some("something failed".to_string()));
    }

    #[test]
    fn test_transient_store_set_status() {
        let mut store = TransientStateStore::new();

        store.set_status("room-1", RoomStatus::Creating);
        store.set_status("room-2", RoomStatus::Deleting);

        assert_eq!(store.get_status("room-1"), Some(&RoomStatus::Creating));
        assert_eq!(store.get_status("room-2"), Some(&RoomStatus::Deleting));
        assert_eq!(store.get_status("room-3"), None);
    }

    #[test]
    fn test_transient_store_set_error() {
        let mut store = TransientStateStore::new();

        store.set_error("room-1", "worktree creation failed".to_string());

        assert_eq!(store.get_status("room-1"), Some(&RoomStatus::Error));
        assert_eq!(store.get_error("room-1"), Some("worktree creation failed"));
    }

    #[test]
    fn test_transient_store_overwrite() {
        let mut store = TransientStateStore::new();

        store.set_status("room-1", RoomStatus::Creating);
        assert_eq!(store.get_status("room-1"), Some(&RoomStatus::Creating));

        store.set_status("room-1", RoomStatus::PostCreateRunning);
        assert_eq!(
            store.get_status("room-1"),
            Some(&RoomStatus::PostCreateRunning)
        );
    }

    #[test]
    fn test_transient_store_remove() {
        let mut store = TransientStateStore::new();

        store.set_status("room-1", RoomStatus::Creating);
        assert!(store.has("room-1"));

        let removed = store.remove("room-1");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().status, RoomStatus::Creating);
        assert!(!store.has("room-1"));

        // Removing again returns None
        assert!(store.remove("room-1").is_none());
    }

    #[test]
    fn test_transient_store_clear() {
        let mut store = TransientStateStore::new();

        store.set_status("room-1", RoomStatus::Creating);
        store.set_status("room-2", RoomStatus::Deleting);
        assert_eq!(store.len(), 2);

        store.clear();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_transient_store_room_names() {
        let mut store = TransientStateStore::new();

        store.set_status("room-a", RoomStatus::Creating);
        store.set_status("room-b", RoomStatus::Deleting);
        store.set_status("room-c", RoomStatus::PostCreateRunning);

        let mut names: Vec<&str> = store.room_names().collect();
        names.sort();

        assert_eq!(names, vec!["room-a", "room-b", "room-c"]);
    }
}
