# Sidebar (Side Navigation)

## Layout

The sidebar occupies the left portion of the screen:
- **Both panels visible**: Fixed 40 columns width
- **Sidebar only**: 100% width
- **Hidden**: 0% width (toggled via `Ctrl+b`)

## Visual Structure

```
┌─ Rooms ─────────────────────┐
│ ACTIVE                      │
│ ● quick-fox-a1b2 [primary]  │
│   └─ main                   │
│                             │
│ INACTIVE                    │
│ ○ calm-bear-1f2c            │
│   └─ bugfix/session         │
│                             │
│ FAILED                      │
│ ! broken-room               │
│   └─ main                   │
└─────────────────────────────┘
```

## Room List Entry

Each room displays:
1. **Status icon**: Indicates current state
2. **Room name**: Worktree directory name
3. **Primary label**: `[primary]` when the item is the primary worktree
4. **Branch name**: Shown on second line with tree connector (`└─`)
5. **Failure reason**: Failed entries include a short label (e.g., `[prunable]`)

### Sections

The list is grouped into sections:
- **ACTIVE**: Worktrees with an attached PTY session
- **INACTIVE**: Worktrees without a PTY session
- **FAILED**: Prunable or error worktrees

Sections only appear if they contain at least one worktree. Worktrees within each
section are listed alphabetically by name.

## Text Overflow

When room names or branch names exceed the available sidebar width, they are truncated with an ellipsis (`…`):
- Room names are truncated after accounting for the status icon prefix (2 characters) and any primary label
- Branch names are truncated after accounting for the tree connector prefix (5 characters)
- Unicode characters are handled correctly using unicode width measurements

## Status Icons

| Icon | Status | Color |
|------|--------|-------|
| `○` | Idle | White |
| `◐` | Creating | Yellow |
| `●` | Ready | Green |
| `!` | Error | Red |
| `?` | Orphaned | Dark Gray |
| `○` | Deleting | White |

Inactive ready rooms display a hollow circle (`○`) instead of a filled circle.

## Focus Indication

| State | Border Color | Selection Highlight |
|-------|--------------|---------------------|
| Focused | Cyan | Cyan background |
| Unfocused | Dark Gray | None |

## Selection Behavior

- Single selection only
- Arrow keys (`j`/`k` or `↑`/`↓`) move selection
- Selection wraps at list boundaries
- Pressing `Enter` focuses the terminal for the selected room
- Selecting a FAILED room does not start a shell; prunable entries trigger a worktree prune

## Empty State

When no rooms exist, display:
```
Press 'a' to create one
```

## Title

The sidebar title is "Rooms" (displayed in the border).

## Scrolling

- List scrolls to keep selected item visible
- No scroll indicator currently shown
