# Sidebar (Side Navigation)

## Layout

The sidebar occupies the left portion of the screen:
- **Both panels visible**: Fixed 40 columns width
- **Sidebar only**: 100% width
- **Hidden**: 0% width (toggled via `Ctrl+b`)

## Visual Structure

```
┌─ Rooms ─────────────────────┐
│ ● quick-fox-a1b2            │
│   └─ feature/auth           │
│                             │
│ ○ calm-bear-1f2c            │
│   └─ bugfix/session         │
│                             │
│ ! broken-room               │
│   └─ main                   │
│                             │
│ "Press 'a' to create one"   │
│ (shown when empty)          │
└─────────────────────────────┘
```

## Room List Entry

Each room displays:
1. **Status icon**: Indicates current state
2. **Room name**: User-given or auto-generated
3. **Branch name**: Shown on second line with tree connector (`└─`)

## Text Overflow

When room names or branch names exceed the available sidebar width, they are truncated with an ellipsis (`…`):
- Room names are truncated after accounting for the status icon prefix (2 characters)
- Branch names are truncated after accounting for the tree connector prefix (5 characters)
- Unicode characters are handled correctly using unicode width measurements

## Status Icons

| Icon | Status | Color |
|------|--------|-------|
| `○` | Idle | White |
| `◐` | Creating | Yellow |
| `◐` | PostCreateRunning | Yellow |
| `●` | Ready | Green |
| `!` | Error | Red |
| `?` | Orphaned | Dark Gray |
| `○` | Deleting | White |

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
