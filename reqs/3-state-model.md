# State Model

## Room Data Structure

### Room Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID v4 | Unique identifier for the room |
| `name` | String | User-given or auto-generated name (1-40 chars, lowercase, alphanumeric + hyphens) |
| `branch` | String | Git branch name |
| `path` | PathBuf | Absolute path to worktree directory |
| `created_at` | DateTime<Utc> | Creation timestamp |
| `last_used_at` | DateTime<Utc> | Last selection timestamp |
| `status` | RoomStatus | Current lifecycle state |
| `last_error` | Option<String> | Error message if status is Error |

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
Any → Orphaned (on startup validation)
```

## State Persistence

### File Location
```
{repo_root}/.rooms/state.json
```

### File Format
```json
{
  "rooms": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "quick-fox-a1b2",
      "branch": "feature/auth",
      "path": "/path/to/repo/.rooms/quick-fox-a1b2",
      "created_at": "2025-01-16T14:23:45Z",
      "last_used_at": "2025-01-16T15:30:00Z",
      "status": "Ready",
      "last_error": null
    }
  ]
}
```

### Atomic Persistence

State file writes MUST follow this pattern:
1. Write content to temporary file (`state.json.tmp`)
2. Rename temporary file to target (`state.json`)

This ensures the state file is never partially written.

### State Operations

| Operation | Trigger |
|-----------|---------|
| Load | Application startup |
| Save | Room creation, deletion, rename, status change |
| Validate | Startup (marks missing worktrees as Orphaned) |

## In-Memory State

The following state is kept in memory only and not persisted:

- PTY sessions (recreated on room selection)
- Panel visibility toggles
- Current selection index
- Focus state
- Active dialogs/prompts
