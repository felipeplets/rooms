use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use uuid::Uuid;

use crate::config::{PostCreateCommand, RunIn};

/// Result of a single post-create command execution.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Name of the command that was run.
    pub name: String,
    /// Whether the command succeeded.
    pub success: bool,
    /// Combined output (stdout + stderr).
    pub output: String,
    /// Exit code if available.
    pub exit_code: Option<i32>,
}

/// Final result of all post-create commands for a room.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PostCreateResult {
    /// The room ID this result is for.
    pub room_id: Uuid,
    /// Results of each command in order.
    pub command_results: Vec<CommandResult>,
    /// Whether all commands succeeded.
    pub success: bool,
    /// Error message if any command failed.
    pub error: Option<String>,
}

/// A handle to a running post-create operation.
pub struct PostCreateHandle {
    /// Room ID this operation is for.
    #[allow(dead_code)]
    pub room_id: Uuid,
    /// Receiver for the final result.
    receiver: Receiver<PostCreateResult>,
}

impl PostCreateHandle {
    /// Check if the operation is complete without blocking.
    /// Returns Some(result) if done, None if still running.
    pub fn try_recv(&self) -> Option<PostCreateResult> {
        self.receiver.try_recv().ok()
    }
}

/// Run post-create commands for a room in a background thread.
///
/// Returns a handle that can be polled for completion.
pub fn run_post_create_commands(
    room_id: Uuid,
    room_path: PathBuf,
    repo_root: PathBuf,
    commands: Vec<PostCreateCommand>,
) -> PostCreateHandle {
    let (tx, rx): (Sender<PostCreateResult>, Receiver<PostCreateResult>) = mpsc::channel();

    thread::spawn(move || {
        let mut command_results = Vec::new();
        let mut all_success = true;
        let mut error_message = None;

        for cmd_config in commands {
            let working_dir = match cmd_config.run_in {
                RunIn::RoomRoot => room_path.clone(),
                RunIn::RepoRoot => repo_root.clone(),
            };

            let result = run_single_command(&cmd_config, &working_dir);

            if !result.success {
                all_success = false;
                error_message = Some(format!(
                    "Command '{}' failed: {}",
                    cmd_config.name,
                    result.output.lines().next().unwrap_or("unknown error")
                ));
            }

            command_results.push(result);

            // Stop on first failure
            if !all_success {
                break;
            }
        }

        let result = PostCreateResult {
            room_id,
            command_results,
            success: all_success,
            error: error_message,
        };

        // Send result (ignore error if receiver dropped)
        let _ = tx.send(result);
    });

    PostCreateHandle {
        room_id,
        receiver: rx,
    }
}

/// Run a single command synchronously.
fn run_single_command(cmd: &PostCreateCommand, working_dir: &PathBuf) -> CommandResult {
    let output = Command::new(&cmd.command)
        .args(&cmd.args)
        .current_dir(working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = if stderr.is_empty() {
                stdout.to_string()
            } else if stdout.is_empty() {
                stderr.to_string()
            } else {
                format!("{}\n{}", stdout, stderr)
            };

            CommandResult {
                name: cmd.name.clone(),
                success: output.status.success(),
                output: combined,
                exit_code: output.status.code(),
            }
        }
        Err(e) => CommandResult {
            name: cmd.name.clone(),
            success: false,
            output: format!("Failed to execute: {}", e),
            exit_code: None,
        },
    }
}
