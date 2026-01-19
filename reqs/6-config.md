# Configuration

## File Location

```
{repo_root}/.rooms/config.toml
```

## Format

TOML format. If the file does not exist, defaults are used.

## Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `base_branch` | String | (none) | Default base branch for new rooms |
| `rooms_dir` | String | `..` | Directory for storing room worktrees |
| `post_create_commands` | Array | `[]` | Commands to run after room creation |

## Post-Create Command Structure

Each post-create command has the following fields:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | String | Yes | Display name for the command |
| `command` | String | Yes | Executable to run |
| `args` | Array<String> | No | Arguments to pass to the command |
| `run_in` | String | No | Where to run: `room_root` (default) or `repo_root` |

### Run Location

| Value | Description |
|-------|-------------|
| `room_root` | Run in the worktree directory |
| `repo_root` | Run in the main repository root |

## Example Configuration

```toml
# Default branch for new rooms
base_branch = "main"

# Directory for room worktrees (relative to primary worktree)
rooms_dir = ".."

# Post-create commands
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

[[post_create_commands]]
name = "init"
command = "./scripts/init.sh"
args = []
run_in = "room_root"
```

## Behavior

- Configuration is loaded once at startup
- Changes require restarting the application
- Invalid configuration results in an error message and exit
- Missing file uses defaults (no base branch, `..` directory, no post-create commands)
