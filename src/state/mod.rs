// Allow dead code for now - these utilities will be used in later implementation steps
#![allow(dead_code)]

mod events;
mod transient;

pub use events::EventLog;
#[allow(unused_imports)]
pub use transient::{TransientRoomState, TransientStateStore};

// Re-export RoomStatus from room::model for backward compatibility
pub use crate::room::RoomStatus;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

/// State file name.
pub const STATE_FILE: &str = "state.json";

#[derive(Error, Debug)]
pub enum StateError {
    #[error("failed to read state file: {0}")]
    Read(#[from] std::io::Error),

    #[error("failed to parse state file: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("failed to create directory: {path}")]
    CreateDir {
        path: String,
        source: std::io::Error,
    },
}

/// A managed workspace backed by a git worktree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    /// Unique identifier for this room.
    pub id: Uuid,

    /// User-given or generated name.
    pub name: String,

    /// Git branch name.
    pub branch: String,

    /// Path to the worktree directory.
    pub path: PathBuf,

    /// When the room was created.
    pub created_at: DateTime<Utc>,

    /// When the room was last used/selected.
    pub last_used_at: DateTime<Utc>,

    /// Current status in the lifecycle.
    #[serde(default)]
    pub status: RoomStatus,

    /// Last error message if status is Error.
    #[serde(default)]
    pub last_error: Option<String>,
}

impl Room {
    /// Create a new room with the given name, branch, and path.
    pub fn new(name: String, branch: String, path: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            branch,
            path,
            created_at: now,
            last_used_at: now,
            status: RoomStatus::Creating,
            last_error: None,
        }
    }

    /// Update the last_used_at timestamp to now.
    pub fn touch(&mut self) {
        self.last_used_at = Utc::now();
    }

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

/// Persistent state for all rooms in a repository.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoomsState {
    /// All tracked rooms.
    #[serde(default)]
    pub rooms: Vec<Room>,
}

impl RoomsState {
    /// Load state from a JSON file.
    ///
    /// Returns empty state if the file doesn't exist.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, StateError> {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(path)?;
        let state: RoomsState = serde_json::from_str(&contents)?;
        Ok(state)
    }

    /// Load state from the default location within a rooms directory.
    pub fn load_from_rooms_dir<P: AsRef<Path>>(rooms_dir: P) -> Result<Self, StateError> {
        let state_path = rooms_dir.as_ref().join(STATE_FILE);
        Self::load(state_path)
    }

    /// Save state to a JSON file atomically.
    ///
    /// Writes to a temporary file first, then renames to ensure atomicity.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), StateError> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| StateError::CreateDir {
                    path: parent.to_string_lossy().to_string(),
                    source: e,
                })?;
            }
        }

        // Write to temp file first
        let temp_path = path.with_extension("json.tmp");
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&temp_path, contents)?;

        // Atomic rename
        fs::rename(&temp_path, path)?;

        Ok(())
    }

    /// Save state to the default location within a rooms directory.
    pub fn save_to_rooms_dir<P: AsRef<Path>>(&self, rooms_dir: P) -> Result<(), StateError> {
        let state_path = rooms_dir.as_ref().join(STATE_FILE);
        self.save(state_path)
    }

    /// Find a room by name.
    pub fn find_by_name(&self, name: &str) -> Option<&Room> {
        self.rooms.iter().find(|r| r.name == name)
    }

    /// Find a room by name (mutable).
    pub fn find_by_name_mut(&mut self, name: &str) -> Option<&mut Room> {
        self.rooms.iter_mut().find(|r| r.name == name)
    }

    /// Find a room by ID.
    pub fn find_by_id(&self, id: Uuid) -> Option<&Room> {
        self.rooms.iter().find(|r| r.id == id)
    }

    /// Add a new room to the state.
    pub fn add_room(&mut self, room: Room) {
        self.rooms.push(room);
    }

    /// Remove a room by name. Returns the removed room if found.
    pub fn remove_by_name(&mut self, name: &str) -> Option<Room> {
        if let Some(idx) = self.rooms.iter().position(|r| r.name == name) {
            Some(self.rooms.remove(idx))
        } else {
            None
        }
    }

    /// Check if a room name already exists.
    pub fn name_exists(&self, name: &str) -> bool {
        self.rooms.iter().any(|r| r.name == name)
    }

    /// Validate rooms against the filesystem.
    ///
    /// Marks rooms as Orphaned if their worktree path doesn't exist.
    /// Returns the number of rooms that were marked as orphaned.
    pub fn validate_paths(&mut self) -> usize {
        let mut orphaned_count = 0;

        for room in &mut self.rooms {
            if !room.path.exists() && room.status != RoomStatus::Orphaned {
                room.status = RoomStatus::Orphaned;
                orphaned_count += 1;
            }
        }

        orphaned_count
    }

    /// Find a room by its path.
    pub fn find_by_path(&self, path: &Path) -> Option<&Room> {
        self.rooms.iter().find(|r| r.path == path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_new() {
        let room = Room::new(
            "test-room".to_string(),
            "test-branch".to_string(),
            PathBuf::from("/path/to/room"),
        );

        assert_eq!(room.name, "test-room");
        assert_eq!(room.branch, "test-branch");
        assert_eq!(room.status, RoomStatus::Creating);
        assert!(room.last_error.is_none());
    }

    #[test]
    fn test_room_set_error() {
        let mut room = Room::new(
            "test".to_string(),
            "test".to_string(),
            PathBuf::from("/test"),
        );

        room.set_error("something went wrong".to_string());

        assert_eq!(room.status, RoomStatus::Error);
        assert_eq!(room.last_error, Some("something went wrong".to_string()));
    }

    #[test]
    fn test_room_set_ready() {
        let mut room = Room::new(
            "test".to_string(),
            "test".to_string(),
            PathBuf::from("/test"),
        );
        room.set_error("error".to_string());
        room.set_ready();

        assert_eq!(room.status, RoomStatus::Ready);
        assert!(room.last_error.is_none());
    }

    #[test]
    fn test_rooms_state_default() {
        let state = RoomsState::default();
        assert!(state.rooms.is_empty());
    }

    #[test]
    fn test_rooms_state_add_and_find() {
        let mut state = RoomsState::default();
        let room = Room::new(
            "my-room".to_string(),
            "my-branch".to_string(),
            PathBuf::from("/rooms/my-room"),
        );
        let room_id = room.id;

        state.add_room(room);

        assert!(state.name_exists("my-room"));
        assert!(!state.name_exists("other-room"));
        assert!(state.find_by_name("my-room").is_some());
        assert!(state.find_by_id(room_id).is_some());
    }

    #[test]
    fn test_rooms_state_remove() {
        let mut state = RoomsState::default();
        state.add_room(Room::new(
            "room1".to_string(),
            "branch1".to_string(),
            PathBuf::from("/r1"),
        ));
        state.add_room(Room::new(
            "room2".to_string(),
            "branch2".to_string(),
            PathBuf::from("/r2"),
        ));

        let removed = state.remove_by_name("room1");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "room1");
        assert!(!state.name_exists("room1"));
        assert!(state.name_exists("room2"));
    }

    #[test]
    fn test_rooms_state_persistence() {
        let temp_dir = tempfile::tempdir().unwrap();
        let state_path = temp_dir.path().join("state.json");

        // Create and save state
        let mut state = RoomsState::default();
        state.add_room(Room::new(
            "persisted-room".to_string(),
            "persisted-branch".to_string(),
            PathBuf::from("/rooms/persisted"),
        ));
        state.save(&state_path).unwrap();

        // Load and verify
        let loaded = RoomsState::load(&state_path).unwrap();
        assert_eq!(loaded.rooms.len(), 1);
        assert_eq!(loaded.rooms[0].name, "persisted-room");
        assert_eq!(loaded.rooms[0].branch, "persisted-branch");
    }

    #[test]
    fn test_rooms_state_load_nonexistent() {
        let state = RoomsState::load("/nonexistent/state.json").unwrap();
        assert!(state.rooms.is_empty());
    }

    #[test]
    fn test_atomic_save_creates_parent_dirs() {
        let temp_dir = tempfile::tempdir().unwrap();
        let nested_path = temp_dir.path().join("a").join("b").join("state.json");

        let state = RoomsState::default();
        state.save(&nested_path).unwrap();

        assert!(nested_path.exists());
    }

    #[test]
    fn test_room_status_serialization() {
        let room = Room::new("test".to_string(), "test".to_string(), PathBuf::from("/t"));

        let json = serde_json::to_string(&room).unwrap();
        assert!(json.contains("\"status\":\"creating\""));

        let parsed: Room = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, RoomStatus::Creating);
    }

    #[test]
    fn test_validate_paths_marks_missing_as_orphaned() {
        let mut state = RoomsState::default();

        // Create a temporary directory that we know exists
        let temp_dir = tempfile::tempdir().unwrap();

        // Add a room with a non-existent path
        let mut room = Room::new(
            "missing-room".to_string(),
            "missing-branch".to_string(),
            PathBuf::from("/this/path/does/not/exist"),
        );
        room.status = RoomStatus::Ready;
        state.add_room(room);

        // Add a room with an existing path (temp directory)
        let mut existing_room = Room::new(
            "existing-room".to_string(),
            "existing-branch".to_string(),
            temp_dir.path().to_path_buf(),
        );
        existing_room.status = RoomStatus::Ready;
        state.add_room(existing_room);

        let orphaned = state.validate_paths();

        assert_eq!(orphaned, 1);
        assert_eq!(
            state.find_by_name("missing-room").unwrap().status,
            RoomStatus::Orphaned
        );
        assert_eq!(
            state.find_by_name("existing-room").unwrap().status,
            RoomStatus::Ready
        );
    }

    #[test]
    fn test_validate_paths_doesnt_double_count() {
        let mut state = RoomsState::default();

        let mut room = Room::new(
            "orphan".to_string(),
            "orphan".to_string(),
            PathBuf::from("/nonexistent"),
        );
        room.status = RoomStatus::Orphaned; // Already orphaned
        state.add_room(room);

        let orphaned = state.validate_paths();
        assert_eq!(orphaned, 0); // Shouldn't count already-orphaned rooms
    }
}
