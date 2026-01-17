# Operations and Logging

## Event Log

### File Location
```
{repo_root}/.rooms/events.log
```

### Format
Plain text, one event per line:
```
{timestamp} | {event_type} | {room_name} | {details}
```

### Event Types

| Event Type | Room Name | Details |
|------------|-----------|---------|
| `roomcreated` | Room name | - |
| `roomdeleted` | Room name | - |
| `roomrenamed` | New name | `{old_name} -> {new_name}` |
| `postcreatstarted` | Room name | `N command(s)` |
| `postcreatcompleted` | Room name | - |
| `postcreatfailed` | Room name | Error message |
| `error` | Room name (optional) | Error message |

### Example Log

```
2025-01-16 14:23:45 UTC | roomcreated | quick-fox-a1b2 | -
2025-01-16 14:23:47 UTC | postcreatstarted | quick-fox-a1b2 | 2 command(s)
2025-01-16 14:23:52 UTC | postcreatcompleted | quick-fox-a1b2 | -
2025-01-16 14:25:10 UTC | roomrenamed | calm-bear-1f2c | quick-fox-a1b2 -> calm-bear-1f2c
2025-01-16 14:26:30 UTC | roomdeleted | calm-bear-1f2c | -
```

## Post-Create Commands

### Execution Model

- Commands run in a background thread
- Commands execute sequentially (in order defined in config)
- Fail-fast: Execution stops on first failure
- Room status set to `PostCreateRunning` during execution

### Per-Command Execution

1. Determine working directory (`room_root` or `repo_root`)
2. Spawn process with command and arguments
3. Capture stdout and stderr
4. Check exit code

### Result Tracking

Each command result includes:
- Command name
- Success/failure status
- Combined output (stdout + stderr)
- Exit code

### Status Updates

| Outcome | Room Status | Action |
|---------|-------------|--------|
| All succeed | `Ready` | Log `postcreatcompleted` |
| Any fails | `Error` | Log `postcreatfailed`, set `last_error` |

### Skipping

Post-create commands can be skipped:
- `--no-post-create` CLI flag
- No commands configured in `config.toml`

## Git Command Wrapper

### Structured Results

All git commands return:
```
{
  stdout: String,
  stderr: String,
  exit_code: i32
}
```

### Error Handling

Git command errors include:
- Command that was executed
- Exit code
- Stderr output
- Actionable error message

### Commands Used

| Operation | Command |
|-----------|---------|
| List worktrees | `git worktree list --porcelain` |
| Create worktree | `git worktree add [-b branch] path [base]` |
| Remove worktree | `git worktree remove path` |
| Move worktree | `git worktree move old_path new_path` |
| Check dirty status | `git status --porcelain` |
| Get repo root | `git rev-parse --show-toplevel` |
| Get current branch | `git rev-parse --abbrev-ref HEAD` |
