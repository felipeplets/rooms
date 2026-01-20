use serde::{Deserialize, Deserializer, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Default directory for rooms worktrees (parent of primary worktree).
pub const DEFAULT_ROOMS_DIR: &str = "..";

/// Configuration file name.
pub const CONFIG_FILE: &str = ".roomsrc.json";

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    Read(#[from] std::io::Error),

    #[error("failed to parse config file: {0}")]
    Parse(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Hooks {
    #[serde(default, deserialize_with = "deserialize_hook_commands")]
    pub post_create: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_hook_commands")]
    pub post_enter: Vec<String>,
}

/// Application configuration loaded from .roomsrc.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Base branch to create new branches from.
    /// Defaults to current HEAD if not specified.
    #[serde(default)]
    pub base_branch: Option<String>,

    /// Directory name for rooms data (relative to primary worktree).
    #[serde(default = "default_rooms_dir")]
    pub rooms_dir: String,

    /// Hooks to run for room lifecycle events.
    #[serde(default)]
    pub hooks: Hooks,
}

fn default_rooms_dir() -> String {
    DEFAULT_ROOMS_DIR.to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_branch: None,
            rooms_dir: default_rooms_dir(),
            hooks: Hooks::default(),
        }
    }
}

impl Config {
    /// Load configuration from a JSON file.
    ///
    /// Returns default config if the file doesn't exist.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&contents)?;
        Ok(config)
    }

    /// Load configuration from the default location within a repository.
    pub fn load_from_primary<P: AsRef<Path>>(primary_worktree: P) -> Result<Self, ConfigError> {
        let config_path = primary_worktree.as_ref().join(CONFIG_FILE);
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

fn deserialize_hook_commands<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(command) => Ok(vec![command]),
        serde_json::Value::Array(items) => {
            let mut commands = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    serde_json::Value::String(command) => commands.push(command),
                    _ => {
                        return Err(serde::de::Error::custom("hook commands must be strings"));
                    }
                }
            }
            Ok(commands)
        }
        serde_json::Value::Null => Ok(Vec::new()),
        _ => Err(serde::de::Error::custom(
            "hook commands must be a string or array of strings",
        )),
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
        assert!(config.hooks.post_create.is_empty());
        assert!(config.hooks.post_enter.is_empty());
    }

    #[test]
    fn test_parse_minimal_config() {
        let json = "{}";
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.rooms_dir, "..");
    }

    #[test]
    fn test_parse_full_config() {
        let json = r#"
{
  "base_branch": "main",
  "rooms_dir": ".worktrees",
  "hooks": {
    "post_create": ["npm install", "make setup"],
    "post_enter": "ls -la"
  }
}
"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.base_branch, Some("main".to_string()));
        assert_eq!(config.rooms_dir, ".worktrees");
        assert_eq!(config.hooks.post_create.len(), 2);
        assert_eq!(config.hooks.post_create[0], "npm install");
        assert_eq!(config.hooks.post_create[1], "make setup");
        assert_eq!(config.hooks.post_enter.len(), 1);
        assert_eq!(config.hooks.post_enter[0], "ls -la");
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let config = Config::load("/nonexistent/path/.roomsrc.json").unwrap();
        assert_eq!(config.rooms_dir, "..");
    }

    #[test]
    fn test_rooms_path() {
        let config = Config::default();
        let path = config.rooms_path("/repo");
        assert_eq!(path, PathBuf::from("/"));
    }

    #[test]
    fn test_deserialize_hook_null() {
        let json = r#"{"hooks": {"post_create": null}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.hooks.post_create.is_empty());
    }

    #[test]
    fn test_deserialize_hook_single_string() {
        let json = r#"{"hooks": {"post_create": "echo hello"}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.hooks.post_create.len(), 1);
        assert_eq!(config.hooks.post_create[0], "echo hello");
    }

    #[test]
    fn test_deserialize_hook_array_of_strings() {
        let json = r#"{"hooks": {"post_create": ["cmd1", "cmd2", "cmd3"]}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.hooks.post_create.len(), 3);
        assert_eq!(config.hooks.post_create[0], "cmd1");
        assert_eq!(config.hooks.post_create[1], "cmd2");
        assert_eq!(config.hooks.post_create[2], "cmd3");
    }

    #[test]
    fn test_deserialize_hook_empty_array() {
        let json = r#"{"hooks": {"post_create": []}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.hooks.post_create.is_empty());
    }

    #[test]
    fn test_deserialize_hook_invalid_type() {
        let json = r#"{"hooks": {"post_create": 123}}"#;
        let result: Result<Config, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_hook_array_with_non_string() {
        let json = r#"{"hooks": {"post_create": ["valid", 123, "also valid"]}}"#;
        let result: Result<Config, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
