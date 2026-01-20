# Room Lifecycle

## Create Room

### Interactive Mode (Key: `a`)

1. **Prompt for room name**
   - Display text input dialog
   - Pre-fill with auto-generated name (user can accept or replace)
   - Validate: 1-40 chars, lowercase, alphanumeric + hyphens

2. **Prompt for branch name**
   - Display text input dialog
   - Pre-fill with room name (sanitized for git branch)
   - User can accept or specify different branch

3. **Execute creation**
   - Create worktree via git in a background task
   - Show a temporary INACTIVE entry with an animated yellow dot and `Creating...` label while creating
   - Refresh worktree list when creation completes
   - Auto-enter the new room (start PTY session)
   - Run `post_create` hooks, then `post_enter` hooks (if configured)

### Quick Mode (Key: `A`)

1. **Auto-generate name**
   - Format: `{adjective}-{noun}-{4hex}`
   - Example: `quick-fox-a1b2`
   - Retries up to 100 times if name exists

2. **Use auto-generated name as branch**
   - Branch name same as room name

3. **Execute creation**
   - Same as interactive mode step 3

### Name Generation

Word lists:
- **Adjectives** (35): quick, lazy, happy, calm, bold, bright, cool, warm, swift, keen, fresh, crisp, gentle, vivid, steady, clever, witty, merry, lively, peaceful, cosmic, lunar, solar, stellar, amber, azure, coral, golden, silver, emerald, rustic, modern, classic, noble, humble
- **Nouns** (34): fox, owl, bear, wolf, hawk, deer, hare, seal, crow, swan, oak, pine, elm, maple, cedar, river, stream, lake, pond, brook, peak, ridge, vale, grove, meadow, stone, crystal, ember, frost, breeze, dawn, dusk, noon, tide, wave

### Git Operations

Depending on branch state:
- **Branch exists**: `git worktree add {path} {branch}`
- **New branch from HEAD**: `git worktree add -b {branch} {path}`
- **New branch from base**: `git worktree add -b {branch} {path} {base_branch}`

### Status Transitions

```
(none) → Creating → Ready
         Creating → Error (if git fails, user can retry or remove)
```

## Delete Room

### Trigger

- **d**: Shows confirmation dialog before deleting
- **D**: Deletes immediately without confirmation (use with caution)

### Confirmation Dialog

Displays:
- Room name
- Worktree path
- Branch name
- Dirty status (if uncommitted changes exist)
- List of first 3-5 modified/untracked files
- Warning: "Branch will NOT be deleted"
- Buttons: [Cancel] [Delete]

### Dirty Status Check

Before showing dialog, check for uncommitted changes:
```
git status --porcelain
```

Report:
- Count of modified files
- Count of untracked files
- Summary of first 5 files

### Execution

1. Run `git worktree remove {path}`
2. Refresh worktree list
3. Log deletion event

### Safety Guarantees

- Git branch is preserved (only worktree removed)
- User must confirm even for clean rooms
- Dirty rooms show explicit warning

## Rename Room

### Trigger
Key: `r` on selected room

### Prompt

- Display text input dialog
- Pre-fill with current room name
- Validate new name (same rules as creation)

### Validation

- New name must differ from current
- New name must not already exist
- Destination path must not exist on disk

### Execution

1. Run `git worktree move {old_path} {new_path}`
2. Refresh worktree list
3. Log rename event
4. Destroy existing PTY session (working directory changed)

### Constraints

- Branch name is NOT changed
- Only room name and worktree path change
