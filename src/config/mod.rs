use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Directory name for rooms config and logs within a repository.
pub const CONFIG_DIR: &str = ".rooms";

/// Default directory for rooms worktrees (parent of primary worktree).
pub const DEFAULT_ROOMS_DIR: &str = "..";

/// Configuration file name.
pub const CONFIG_FILE: &str = "config.toml";

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    Read(#[from] std::io::Error),

    #[error("failed to parse config file: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("failed to serialize config: {0}")]
    Serialize(#[from] toml::ser::Error),
}

/// Post-create command configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PostCreateCommand {
    /// Display name for this command.
    pub name: String,

    /// The command to run.
    pub command: String,

    /// Arguments to pass to the command.
    #[serde(default)]
    pub args: Vec<String>,

    /// Where to run the command.
    #[serde(default)]
    pub run_in: RunIn,
}

/// Where to run a post-create command.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RunIn {
    /// Run in the room's worktree root.
    #[default]
    RoomRoot,

    /// Run in the main repository root.
    RepoRoot,
}

/// Application configuration loaded from config.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Base branch to create new branches from.
    /// Defaults to current HEAD if not specified.
    #[serde(default)]
    pub base_branch: Option<String>,

    /// Directory name for rooms data (relative to primary worktree).
    #[serde(default = "default_rooms_dir")]
    pub rooms_dir: String,

    /// Commands to run after creating a new room.
    #[serde(default)]
    pub post_create_commands: Vec<PostCreateCommand>,
}

fn default_rooms_dir() -> String {
    DEFAULT_ROOMS_DIR.to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_branch: None,
            rooms_dir: default_rooms_dir(),
            post_create_commands: Vec::new(),
        }
    }
}

impl Config {
    /// Load configuration from a TOML file.
    ///
    /// Returns default config if the file doesn't exist.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Load configuration from the default location within a repository.
    pub fn load_from_repo<P: AsRef<Path>>(repo_root: P) -> Result<Self, ConfigError> {
        let config_path = repo_root.as_ref().join(CONFIG_DIR).join(CONFIG_FILE);
        Self::load(config_path)
    }

    /// Get the full path to the rooms directory.
    pub fn rooms_path<P: AsRef<Path>>(&self, primary_worktree: P) -> PathBuf {
        let primary = primary_worktree.as_ref();
        if self.rooms_dir == ".." {
            return primary
                .parent()
                .map(|parent| parent.to_path_buf())
                // Edge case: If primary_worktree has no parent (e.g., at filesystem root),
                // fall back to using the primary_worktree itself. This is a rare scenario
                // but allows the application to continue functioning.
                .unwrap_or_else(|| primary.to_path_buf());
        }

        let rooms_path = PathBuf::from(&self.rooms_dir);
        if rooms_path.is_absolute() {
            rooms_path
        } else {
            primary.join(rooms_path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.rooms_dir, "..");
        assert!(config.base_branch.is_none());
        assert!(config.post_create_commands.is_empty());
    }

    #[test]
    fn test_parse_minimal_config() {
        let toml = "";
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.rooms_dir, "..");
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
base_branch = "main"
rooms_dir = ".worktrees"

[[post_create_commands]]
name = "install"
command = "npm"
args = ["install"]
run_in = "room_root"

[[post_create_commands]]
name = "setup"
command = "make"
args = ["setup"]
run_in = "repo_root"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.base_branch, Some("main".to_string()));
        assert_eq!(config.rooms_dir, ".worktrees");
        assert_eq!(config.post_create_commands.len(), 2);

        let cmd1 = &config.post_create_commands[0];
        assert_eq!(cmd1.name, "install");
        assert_eq!(cmd1.command, "npm");
        assert_eq!(cmd1.args, vec!["install"]);
        assert_eq!(cmd1.run_in, RunIn::RoomRoot);

        let cmd2 = &config.post_create_commands[1];
        assert_eq!(cmd2.name, "setup");
        assert_eq!(cmd2.run_in, RunIn::RepoRoot);
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let config = Config::load("/nonexistent/path/config.toml").unwrap();
        assert_eq!(config.rooms_dir, "..");
    }

    #[test]
    fn test_rooms_path() {
        let config = Config::default();
        let path = config.rooms_path("/repo");
        assert_eq!(path, PathBuf::from("/"));
    }
}
