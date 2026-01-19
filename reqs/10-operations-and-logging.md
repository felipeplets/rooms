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

- Hook commands are written to the PTY shell sequentially (in configured order)
- Each command is sent as a line to the shell
- Hooks only run when a PTY session is active for the room

### Hooks

Hooks are executed in the PTY shell:
- `post_create` runs after room creation (before `post_enter`)
- `post_enter` runs when entering a room

### Skipping

Hooks can be skipped:
- `--no-hooks` CLI flag
- No hooks configured in `.roomsrc.json`

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
