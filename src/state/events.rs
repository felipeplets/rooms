use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

/// Event log file name.
pub const EVENTS_FILE: &str = "events.log";

/// Types of events that can be logged.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// A room was created.
    RoomCreated,
    /// A room was deleted.
    RoomDeleted,
    /// A room was renamed.
    RoomRenamed,
    /// Post-create commands started.
    PostCreateStarted,
    /// Post-create commands completed successfully.
    PostCreateCompleted,
    /// Post-create commands failed.
    PostCreateFailed,
    /// An error occurred.
    Error,
}

/// A single event in the log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
    /// Type of event.
    pub event_type: EventType,
    /// Name of the room involved (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_name: Option<String>,
    /// Additional details about the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl Event {
    /// Create a new event with the current timestamp.
    pub fn new(event_type: EventType) -> Self {
        Self {
            timestamp: Utc::now(),
            event_type,
            room_name: None,
            details: None,
        }
    }

    /// Set the room name for this event.
    pub fn with_room(mut self, name: impl Into<String>) -> Self {
        self.room_name = Some(name.into());
        self
    }

    /// Set additional details for this event.
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

/// Event logger for appending events to a log file.
pub struct EventLog {
    log_path: std::path::PathBuf,
}

impl EventLog {
    /// Create a new event log for the given rooms directory.
    pub fn new<P: AsRef<Path>>(rooms_dir: P) -> Self {
        Self {
            log_path: rooms_dir.as_ref().join(EVENTS_FILE),
        }
    }

    /// Append an event to the log file.
    ///
    /// Creates the file if it doesn't exist.
    pub fn log(&self, event: Event) -> std::io::Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.log_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        let mut writer = BufWriter::new(file);

        // Format: timestamp | event_type | room_name | details
        let room = event.room_name.as_deref().unwrap_or("-");
        let details = event.details.as_deref().unwrap_or("-");
        let event_str = format!("{:?}", event.event_type).to_lowercase();

        writeln!(
            writer,
            "{} | {} | {} | {}",
            event.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            event_str,
            room,
            details
        )?;

        writer.flush()
    }

    /// Log a room creation event.
    pub fn log_room_created(&self, room_name: &str) {
        let event = Event::new(EventType::RoomCreated).with_room(room_name);
        let _ = self.log(event);
    }

    /// Log a room deletion event.
    pub fn log_room_deleted(&self, room_name: &str) {
        let event = Event::new(EventType::RoomDeleted).with_room(room_name);
        let _ = self.log(event);
    }

    /// Log a room rename event.
    pub fn log_room_renamed(&self, old_name: &str, new_name: &str) {
        let event = Event::new(EventType::RoomRenamed)
            .with_room(new_name)
            .with_details(format!("{} -> {}", old_name, new_name));
        let _ = self.log(event);
    }

    /// Log post-create commands starting.
    pub fn log_post_create_started(&self, room_name: &str, command_count: usize) {
        let event = Event::new(EventType::PostCreateStarted)
            .with_room(room_name)
            .with_details(format!("{} command(s)", command_count));
        let _ = self.log(event);
    }

    /// Log post-create commands completed successfully.
    pub fn log_post_create_completed(&self, room_name: &str) {
        let event = Event::new(EventType::PostCreateCompleted).with_room(room_name);
        let _ = self.log(event);
    }

    /// Log post-create commands failed.
    pub fn log_post_create_failed(&self, room_name: &str, error: &str) {
        let event = Event::new(EventType::PostCreateFailed)
            .with_room(room_name)
            .with_details(error);
        let _ = self.log(event);
    }

    /// Log an error event.
    pub fn log_error(&self, room_name: Option<&str>, error: &str) {
        let mut event = Event::new(EventType::Error).with_details(error);
        if let Some(name) = room_name {
            event = event.with_room(name);
        }
        let _ = self.log(event);
    }
}
