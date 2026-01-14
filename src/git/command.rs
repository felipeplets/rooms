// Allow dead code for now - these utilities will be used in later implementation steps
#![allow(dead_code)]

use std::path::Path;
use std::process::Command;
use thiserror::Error;

/// Structured result from a subprocess execution.
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl CommandResult {
    /// Returns true if the command exited successfully (code 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Errors that can occur when running git commands.
#[derive(Error, Debug)]
pub enum CommandError {
    #[error("failed to execute '{command}': {message}")]
    ExecutionFailed { command: String, message: String },

    #[error("git command failed (exit code {exit_code}): {message}")]
    GitFailed {
        exit_code: i32,
        message: String,
        stderr: String,
    },

    #[error("not a git repository: {path}")]
    NotAGitRepo { path: String },
}

/// Builder for executing git commands with structured results.
pub struct GitCommand {
    args: Vec<String>,
    working_dir: Option<String>,
}

impl GitCommand {
    /// Create a new git command with the given subcommand.
    pub fn new(subcommand: &str) -> Self {
        Self {
            args: vec![subcommand.to_string()],
            working_dir: None,
        }
    }

    /// Add an argument to the command.
    pub fn arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }

    /// Add multiple arguments to the command.
    pub fn args(mut self, args: &[&str]) -> Self {
        self.args.extend(args.iter().map(|s| s.to_string()));
        self
    }

    /// Set the working directory for the command.
    pub fn current_dir<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.working_dir = Some(dir.as_ref().to_string_lossy().to_string());
        self
    }

    /// Execute the command and return a structured result.
    pub fn run(self) -> Result<CommandResult, CommandError> {
        let mut cmd = Command::new("git");
        cmd.args(&self.args);

        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        let output = cmd.output().map_err(|e| CommandError::ExecutionFailed {
            command: format!("git {}", self.args.join(" ")),
            message: e.to_string(),
        })?;

        let result = CommandResult {
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        };

        Ok(result)
    }

    /// Execute the command and return an error if it fails.
    pub fn run_checked(self) -> Result<CommandResult, CommandError> {
        let cmd_str = format!("git {}", self.args.join(" "));
        let result = self.run()?;

        if !result.success() {
            return Err(CommandError::GitFailed {
                exit_code: result.exit_code,
                message: cmd_str,
                stderr: result.stderr.clone(),
            });
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_version() {
        let result = GitCommand::new("--version").run();
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success());
        assert!(result.stdout.contains("git version"));
    }

    #[test]
    fn test_command_result_success() {
        let result = CommandResult {
            stdout: "output".to_string(),
            stderr: String::new(),
            exit_code: 0,
        };
        assert!(result.success());

        let failed = CommandResult {
            stdout: String::new(),
            stderr: "error".to_string(),
            exit_code: 1,
        };
        assert!(!failed.success());
    }
}
