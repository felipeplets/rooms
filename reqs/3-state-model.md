# State Model

## Worktree View Model

### RoomInfo Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | String | Worktree directory name |
| `branch` | Option<String> | Git branch name (None if detached) |
| `path` | PathBuf | Absolute path to worktree directory |
| `status` | RoomStatus | Current lifecycle state (derived, may be overridden by transient state) |
| `is_prunable` | bool | Worktree marked prunable by Git |
| `last_error` | Option<String> | Error message if status is Error |
| `is_primary` | bool | Primary worktree indicator |

### RoomStatus Enum

| Status | Description |
|--------|-------------|
| `Idle` | Room exists, no background operations in progress |
| `Creating` | Creating worktree and branch via git |
| `PostCreateRunning` | Running configured post-create commands |
| `Ready` | Terminal session active and ready |
| `Error` | Last operation failed (see `last_error`) |
| `Deleting` | Removing worktree |
| `Orphaned` | Worktree directory missing on disk |

### Status Transitions

```
Creating → PostCreateRunning → Ready
Creating → Error
PostCreateRunning → Error
Ready → Deleting → (removed)
Ready → Orphaned (prunable worktree)
```

## Persistence

Rooms state is derived from `git worktree list --porcelain` on each refresh.
No persistent room state file is stored.

## In-Memory State

The following state is kept in memory only and not persisted:

- Transient room status overrides (creating, post-create, error)
- PTY sessions (recreated on room selection)
- Panel visibility toggles
- Current selection index
- Focus state
- Active dialogs/prompts
