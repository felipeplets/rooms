use std::path::PathBuf;

use super::command::{CommandError, GitCommand};

/// Get the root directory of the current git repository.
///
/// Runs `git rev-parse --show-toplevel` from the current working directory.
///
/// # Errors
///
/// Returns an error if:
/// - Not inside a git repository
/// - Git command fails to execute
pub fn get_repo_root() -> Result<PathBuf, CommandError> {
    let result = GitCommand::new("rev-parse").arg("--show-toplevel").run()?;

    if !result.success() {
        return Err(CommandError::NotAGitRepo {
            path: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
        });
    }

    Ok(PathBuf::from(&result.stdout))
}

/// Get the root directory of the git repository containing the given path.
///
/// # Errors
///
/// Returns an error if:
/// - The path is not inside a git repository
/// - Git command fails to execute
#[allow(dead_code)] // Used in tests; will be used in later implementation steps
pub fn get_repo_root_from<P: AsRef<std::path::Path>>(path: P) -> Result<PathBuf, CommandError> {
    let result = GitCommand::new("rev-parse")
        .arg("--show-toplevel")
        .current_dir(path.as_ref())
        .run()?;

    if !result.success() {
        return Err(CommandError::NotAGitRepo {
            path: path.as_ref().to_string_lossy().to_string(),
        });
    }

    Ok(PathBuf::from(&result.stdout))
}

/// Get the primary worktree path for the repository at the given path.
///
/// Runs `git rev-parse --path-format=absolute --git-common-dir` and trims the
/// trailing `/.git` from the result to return the primary worktree directory.
///
/// # Errors
///
/// Returns an error if:
/// - The path is not inside a git repository
/// - Git command fails to execute
pub fn get_primary_worktree_path_from<P: AsRef<std::path::Path>>(
    repo_root: P,
) -> Result<PathBuf, CommandError> {
    let result = GitCommand::new("rev-parse")
        .args(&["--path-format=absolute", "--git-common-dir"])
        .current_dir(repo_root.as_ref())
        .run()?;

    if !result.success() {
        return Err(CommandError::NotAGitRepo {
            path: repo_root.as_ref().to_string_lossy().to_string(),
        });
    }

    let mut common_dir = PathBuf::from(result.stdout);
    if common_dir.file_name().and_then(|n| n.to_str()) == Some(".git") {
        if let Some(parent) = common_dir.parent() {
            common_dir = parent.to_path_buf();
        }
    }

    Ok(common_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_repo_root_in_git_repo() {
        // This test runs within the rooms repo, so it should succeed
        let result = get_repo_root();
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists());
        assert!(path.join(".git").exists());
    }

    #[test]
    fn test_get_repo_root_from_subdirectory() {
        // Get repo root, then test from src/ subdirectory
        let root = get_repo_root().expect("should be in a git repo");
        let src_dir = root.join("src");

        if src_dir.exists() {
            let result = get_repo_root_from(&src_dir);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), root);
        }
    }

    #[test]
    fn test_get_repo_root_from_fresh_git_repo() {
        use std::process::Command;

        // Create a temp directory and initialize a git repo
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let temp_path = temp_dir.path();

        // Initialize git repo
        let status = Command::new("git")
            .args(["init"])
            .current_dir(temp_path)
            .output()
            .expect("failed to run git init");
        assert!(status.status.success(), "git init failed");

        // Test detection
        let result = get_repo_root_from(temp_path);
        assert!(result.is_ok());

        // Canonicalize both paths to handle symlinks (e.g., /tmp -> /private/tmp on macOS)
        let detected = result.unwrap().canonicalize().unwrap();
        let expected = temp_path.canonicalize().unwrap();
        assert_eq!(detected, expected);
    }

    #[test]
    fn test_get_repo_root_from_non_git_directory() {
        // Create a temp directory WITHOUT git init
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let result = get_repo_root_from(temp_dir.path());

        assert!(result.is_err());
        match result.unwrap_err() {
            super::CommandError::NotAGitRepo { .. } => (),
            e => panic!("expected NotAGitRepo error, got: {e}"),
        }
    }

    #[test]
    fn test_get_repo_root_from_nested_subdirectory() {
        use std::fs;
        use std::process::Command;

        // Create temp git repo with nested directories
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let temp_path = temp_dir.path();

        Command::new("git")
            .args(["init"])
            .current_dir(temp_path)
            .output()
            .expect("failed to run git init");

        // Create nested structure: repo/a/b/c
        let nested = temp_path.join("a").join("b").join("c");
        fs::create_dir_all(&nested).expect("failed to create nested dirs");

        // Detection from nested should return repo root
        let result = get_repo_root_from(&nested);
        assert!(result.is_ok());

        let detected = result.unwrap().canonicalize().unwrap();
        let expected = temp_path.canonicalize().unwrap();
        assert_eq!(detected, expected);
    }

    #[test]
    fn test_get_primary_worktree_path_from_repo_root() {
        use std::process::Command;

        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let temp_path = temp_dir.path();

        Command::new("git")
            .args(["init"])
            .current_dir(temp_path)
            .output()
            .expect("failed to run git init");

        let result = get_primary_worktree_path_from(temp_path);
        assert!(result.is_ok());

        let detected = result.unwrap().canonicalize().unwrap();
        let expected = temp_path.canonicalize().unwrap();
        assert_eq!(detected, expected);
    }
}
