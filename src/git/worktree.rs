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
}

impl Worktree {
    /// Check if this worktree's directory exists on disk.
    pub fn exists(&self) -> bool {
        self.path.exists()
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
/// ```
fn parse_porcelain_output(output: &str) -> Vec<Worktree> {
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_head: Option<String> = None;
    let mut current_branch: Option<String> = None;
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
}
