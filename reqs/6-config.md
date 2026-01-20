# Configuration

## File Location

```
{primary_worktree_root}/.roomsrc.json
```

## Format

JSON format. If the file does not exist, defaults are used.

## Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `base_branch` | String | (none) | Default base branch for new rooms |
| `rooms_dir` | String | `..` | Directory for storing room worktrees |
| `hooks` | Object | `{}` | Lifecycle hooks (post-create and post-enter) |

## Hooks

Hooks are strings or arrays of strings. Each string is a command sent to the room's PTY shell.

Supported keys:
- `post_create`: runs immediately after creating a room
- `post_enter`: runs when a room's PTY session is created (including after create)

## Example Configuration

```json
{
  "base_branch": "main",
  "rooms_dir": "..",
  "hooks": {
    "post_create": [
      "npm install",
      "make setup"
    ],
    "post_enter": "ls -la"
  }
}
```

## Behavior

- Configuration is loaded once at startup
- Changes require restarting the application
- Invalid configuration results in an error message and exit
- Missing file uses defaults (no base branch, `..` directory, no hooks)
