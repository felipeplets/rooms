// Allow dead code for now - these utilities will be used in later implementation steps
#![allow(dead_code)]

use std::path::PathBuf;

use super::command::{CommandError, GitCommand};

/// Information about a git worktree.
#[derive(Debug, Clone, PartialEq)]
pub struct Worktree {
    /// Path to the worktree directory.
    pub path: PathBuf,

    /// Current HEAD commit SHA.
    pub head: String,

    /// Branch name (without refs/heads/ prefix), or None if detached.
    pub branch: Option<String>,

    /// Whether this is the main worktree.
    pub is_main: bool,

    /// Reason why this worktree is prunable (directory missing, etc.), if any.
    pub prunable: Option<String>,

    /// Reason why this worktree is locked, if any.
    pub locked: Option<String>,
}

impl Worktree {
    /// Check if this worktree's directory exists on disk.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Get the directory name of this worktree (last component of path).
    pub fn name(&self) -> Option<&str> {
        self.path.file_name().and_then(|s| s.to_str())
    }

    /// Check if this worktree is marked as prunable by git.
    pub fn is_prunable(&self) -> bool {
        self.prunable.is_some()
    }

    /// Check if this worktree is locked.
    pub fn is_locked(&self) -> bool {
        self.locked.is_some()
    }
}

/// List all worktrees in the repository.
///
/// Runs `git worktree list --porcelain` and parses the output.
pub fn list_worktrees() -> Result<Vec<Worktree>, CommandError> {
    let result = GitCommand::new("worktree")
        .args(&["list", "--porcelain"])
        .run_checked()?;

    Ok(parse_porcelain_output(&result.stdout))
}

/// List all worktrees from a specific repository.
pub fn list_worktrees_from<P: AsRef<std::path::Path>>(
    repo_path: P,
) -> Result<Vec<Worktree>, CommandError> {
    let result = GitCommand::new("worktree")
        .args(&["list", "--porcelain"])
        .current_dir(repo_path)
        .run_checked()?;

    Ok(parse_porcelain_output(&result.stdout))
}

/// Parse the porcelain output from `git worktree list --porcelain`.
///
/// Format:
/// ```text
/// worktree /path/to/worktree
/// HEAD <sha>
/// branch refs/heads/main
///
/// worktree /path/to/another
/// HEAD <sha>
/// detached
///
/// worktree /path/to/missing
/// HEAD <sha>
/// branch refs/heads/feature
/// prunable gitdir file points to non-existent location
///
/// worktree /path/to/locked
/// HEAD <sha>
/// branch refs/heads/wip
/// locked reason for lock
/// ```
fn parse_porcelain_output(output: &str) -> Vec<Worktree> {
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_head: Option<String> = None;
    let mut current_branch: Option<String> = None;
    let mut current_prunable: Option<String> = None;
    let mut current_locked: Option<String> = None;
    let mut is_first = true;

    for line in output.lines() {
        if line.is_empty() {
            // End of current worktree entry
            if let (Some(path), Some(head)) = (current_path.take(), current_head.take()) {
                worktrees.push(Worktree {
                    path,
                    head,
                    branch: current_branch.take(),
                    is_main: is_first,
                    prunable: current_prunable.take(),
                    locked: current_locked.take(),
                });
                is_first = false;
            }
            continue;
        }

        if let Some(path) = line.strip_prefix("worktree ") {
            current_path = Some(PathBuf::from(path));
        } else if let Some(sha) = line.strip_prefix("HEAD ") {
            current_head = Some(sha.to_string());
        } else if let Some(branch_ref) = line.strip_prefix("branch ") {
            // Strip refs/heads/ prefix
            let branch = branch_ref.strip_prefix("refs/heads/").unwrap_or(branch_ref);
            current_branch = Some(branch.to_string());
        } else if let Some(reason) = line.strip_prefix("prunable ") {
            current_prunable = Some(reason.to_string());
        } else if line == "prunable" {
            // prunable without a reason
            current_prunable = Some(String::new());
        } else if let Some(reason) = line.strip_prefix("locked ") {
            current_locked = Some(reason.to_string());
        } else if line == "locked" {
            // locked without a reason
            current_locked = Some(String::new());
        }
        // "detached" line means no branch, which is the default None
    }

    // Don't forget the last entry (output may not end with blank line)
    if let (Some(path), Some(head)) = (current_path, current_head) {
        worktrees.push(Worktree {
            path,
            head,
            branch: current_branch,
            is_main: is_first,
            prunable: current_prunable,
            locked: current_locked,
        });
    }

    worktrees
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_worktree() {
        let output = "worktree /home/user/repo\nHEAD abc123\nbranch refs/heads/main\n";
        let worktrees = parse_porcelain_output(output);

        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].path, PathBuf::from("/home/user/repo"));
        assert_eq!(worktrees[0].head, "abc123");
        assert_eq!(worktrees[0].branch, Some("main".to_string()));
        assert!(worktrees[0].is_main);
        assert!(!worktrees[0].is_prunable());
        assert!(!worktrees[0].is_locked());
    }

    #[test]
    fn test_parse_multiple_worktrees() {
        let output = r#"worktree /home/user/repo
HEAD abc123
branch refs/heads/main

worktree /home/user/repo/.rooms/feature-x
HEAD def456
branch refs/heads/feature-x
"#;
        let worktrees = parse_porcelain_output(output);

        assert_eq!(worktrees.len(), 2);

        assert_eq!(worktrees[0].path, PathBuf::from("/home/user/repo"));
        assert!(worktrees[0].is_main);

        assert_eq!(
            worktrees[1].path,
            PathBuf::from("/home/user/repo/.rooms/feature-x")
        );
        assert_eq!(worktrees[1].branch, Some("feature-x".to_string()));
        assert!(!worktrees[1].is_main);
    }

    #[test]
    fn test_parse_detached_head() {
        let output = "worktree /home/user/repo\nHEAD abc123\ndetached\n";
        let worktrees = parse_porcelain_output(output);

        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].branch, None);
    }

    #[test]
    fn test_parse_no_trailing_newline() {
        let output = "worktree /home/user/repo\nHEAD abc123\nbranch refs/heads/main";
        let worktrees = parse_porcelain_output(output);

        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].path, PathBuf::from("/home/user/repo"));
    }

    #[test]
    fn test_list_worktrees_in_current_repo() {
        // This test runs in the rooms repo
        let worktrees = list_worktrees();
        assert!(worktrees.is_ok());

        let worktrees = worktrees.unwrap();
        assert!(!worktrees.is_empty());
        assert!(worktrees[0].is_main);
        assert!(worktrees[0].path.exists());
    }

    #[test]
    fn test_list_worktrees_from_path() {
        use std::process::Command;

        // Create a temp git repo
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path();

        Command::new("git")
            .args(["init"])
            .current_dir(temp_path)
            .output()
            .unwrap();

        // Need at least one commit for worktree list to work properly
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(temp_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(temp_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(temp_path)
            .output()
            .unwrap();

        let worktrees = list_worktrees_from(temp_path);
        assert!(worktrees.is_ok());

        let worktrees = worktrees.unwrap();
        assert_eq!(worktrees.len(), 1);
        assert!(worktrees[0].is_main);
    }

    #[test]
    fn test_parse_prunable_worktree_with_reason() {
        let output = r#"worktree /home/user/repo
HEAD abc123
branch refs/heads/main

worktree /home/user/repo/.rooms/missing
HEAD def456
branch refs/heads/feature
prunable gitdir file points to non-existent location
"#;
        let worktrees = parse_porcelain_output(output);

        assert_eq!(worktrees.len(), 2);

        // First worktree is not prunable
        assert!(!worktrees[0].is_prunable());

        // Second worktree is prunable with a reason
        assert!(worktrees[1].is_prunable());
        assert_eq!(
            worktrees[1].prunable,
            Some("gitdir file points to non-existent location".to_string())
        );
    }

    #[test]
    fn test_parse_prunable_worktree_without_reason() {
        let output = "worktree /home/user/repo/.rooms/orphan\nHEAD abc123\nbranch refs/heads/test\nprunable\n";
        let worktrees = parse_porcelain_output(output);

        assert_eq!(worktrees.len(), 1);
        assert!(worktrees[0].is_prunable());
        assert_eq!(worktrees[0].prunable, Some(String::new()));
    }

    #[test]
    fn test_parse_locked_worktree_with_reason() {
        let output = r#"worktree /home/user/repo
HEAD abc123
branch refs/heads/main

worktree /home/user/repo/.rooms/locked-wt
HEAD def456
branch refs/heads/wip
locked working on important changes
"#;
        let worktrees = parse_porcelain_output(output);

        assert_eq!(worktrees.len(), 2);

        // First worktree is not locked
        assert!(!worktrees[0].is_locked());

        // Second worktree is locked with a reason
        assert!(worktrees[1].is_locked());
        assert_eq!(
            worktrees[1].locked,
            Some("working on important changes".to_string())
        );
    }

    #[test]
    fn test_parse_locked_worktree_without_reason() {
        let output =
            "worktree /home/user/repo/.rooms/locked\nHEAD abc123\nbranch refs/heads/test\nlocked\n";
        let worktrees = parse_porcelain_output(output);

        assert_eq!(worktrees.len(), 1);
        assert!(worktrees[0].is_locked());
        assert_eq!(worktrees[0].locked, Some(String::new()));
    }

    #[test]
    fn test_parse_prunable_and_locked_worktree() {
        // Git can report both prunable and locked for the same worktree
        let output = "worktree /home/user/repo/.rooms/both\nHEAD abc123\nbranch refs/heads/test\nprunable missing directory\nlocked prevent cleanup\n";
        let worktrees = parse_porcelain_output(output);

        assert_eq!(worktrees.len(), 1);
        assert!(worktrees[0].is_prunable());
        assert!(worktrees[0].is_locked());
        assert_eq!(worktrees[0].prunable, Some("missing directory".to_string()));
        assert_eq!(worktrees[0].locked, Some("prevent cleanup".to_string()));
    }

    #[test]
    fn test_worktree_name() {
        let worktree = Worktree {
            path: PathBuf::from("/home/user/repo/.rooms/quick-fox-a1b2"),
            head: "abc123".to_string(),
            branch: Some("quick-fox-a1b2".to_string()),
            is_main: false,
            prunable: None,
            locked: None,
        };

        assert_eq!(worktree.name(), Some("quick-fox-a1b2"));
    }

    #[test]
    fn test_worktree_name_main() {
        let worktree = Worktree {
            path: PathBuf::from("/home/user/repo"),
            head: "abc123".to_string(),
            branch: Some("main".to_string()),
            is_main: true,
            prunable: None,
            locked: None,
        };

        assert_eq!(worktree.name(), Some("repo"));
    }

    #[test]
    fn test_worktree_name_root_path() {
        let worktree = Worktree {
            path: PathBuf::from("/"),
            head: "abc123".to_string(),
            branch: Some("main".to_string()),
            is_main: true,
            prunable: None,
            locked: None,
        };

        // Root path has no file_name
        assert_eq!(worktree.name(), None);
    }
}
