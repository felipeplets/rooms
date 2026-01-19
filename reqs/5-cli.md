# Command-Line Interface

## Usage

```
rooms [OPTIONS]
```

## Options

| Flag | Description |
|------|-------------|
| `-h`, `--help` | Print help information |
| `-V`, `--version` | Print version information |
| `--no-hooks` | Skip lifecycle hooks for this session |
| `--no-post-create` | Alias for `--no-hooks` |
| `--debug-pty` | Enable PTY debug logging to `~/.rooms/debug.log` |

## Startup Behavior

1. Parse command-line arguments
2. If `--debug-pty`: Initialize PTY debug logging
3. Verify current directory is within a Git repository (`git rev-parse --show-toplevel`)
4. Detect primary worktree path (`git rev-parse --path-format=absolute --git-common-dir`, trim `/.git`)
5. Load configuration from `{primary_worktree_root}/.roomsrc.json`
6. Discover existing worktrees via `git worktree list --porcelain`
7. Merge transient in-memory status into discovered worktrees
8. Launch TUI

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Normal exit |
| 1 | Error (not a git repository, initialization failure) |

## Environment Variables

| Variable | Usage |
|----------|-------|
| `SHELL` | Shell to spawn in PTY sessions (fallback: `/bin/sh`) |
