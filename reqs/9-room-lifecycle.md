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
   - Create worktree via git
   - Add room to state
   - Run post-create commands (if configured)

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
(none) → Creating → PostCreateRunning → Ready
                  → Error (if git fails)
         PostCreateRunning → Error (if commands fail)
```

## Delete Room

### Trigger

- **Delete key**: Shows confirmation dialog before deleting
- **Ctrl+Delete**: Deletes immediately without confirmation (use with caution)

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
2. Remove room from state
3. Save state
4. Log deletion event

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
2. Update room name and path in state
3. Save state
4. Log rename event
5. Destroy existing PTY session (working directory changed)

### Constraints

- Branch name is NOT changed
- Only room name and worktree path change
