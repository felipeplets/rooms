# Pseudoterminal (PTY)

## Overview

Each room has an associated PTY session that provides an embedded terminal. The session is created when the user first focuses a room and persists until the application exits or the room is deleted/renamed.

## PTY Creation

### Trigger
- User presses `Enter` on a room in the sidebar
- Room does not already have an active session

### Parameters
- **Columns/Rows**: Derived from terminal dimensions
- **Working Directory**: Room's worktree path
- **Shell**: Value of `$SHELL` environment variable, fallback to `/bin/sh`

### Implementation
- Uses `portable-pty` crate for cross-platform PTY support
- Spawns shell process attached to PTY
- Background thread reads PTY output continuously

## Terminal Emulation

### Parser
- Uses `vt100` crate for ANSI/VT100 escape sequence parsing
- Scrollback buffer: 1000 lines

### Supported Features
- ANSI escape sequences
- 256-color palette
- RGB/TrueColor
- Text attributes (bold, italic, underline, etc.)
- Inverse video is rendered even when fg/bg are defaults (cursor visibility)
- Cursor positioning
- Cursor visibility
- Scrollback navigation (1000 lines)

## Scrollback Navigation

Users can view terminal history beyond the visible screen:

### Mouse Scrolling
- **Scroll Up**: View older content (3 lines per scroll)
- **Scroll Down**: Return to newer content (3 lines per scroll)

### Keyboard Scrolling
| Key | Action |
|-----|--------|
| `PageUp` | Scroll up by one page (screen height) |
| `PageDown` | Scroll down by one page (screen height) |

### Behavior
- Scrollback indicator `[↑N]` appears in title when scrolled (N = lines from bottom)
- Cursor is hidden when viewing history
- Scrollback automatically resets to bottom (0) when:
  - User types any key
  - Switching to a different room
- Maximum scrollback: 1000 lines

## Input Handling

### Key Translation
All keyboard input in terminal mode is translated to appropriate sequences:

| Input | Output |
|-------|--------|
| Printable characters | Raw bytes |
| Enter | `\r` |
| Backspace | `0x7f` |
| Tab | `\t` |
| Arrow keys | VT100 escape sequences |
| Function keys | VT100 sequences |
| Ctrl+letter | ASCII control codes (1-26) |

### Reserved Keys
These keys are NOT forwarded to the PTY:
- `Esc` (returns to sidebar)
- `Ctrl+b` (toggles sidebar)
- `PageUp` / `PageDown` (scrollback navigation)

## Selection & Context Menu

### Mouse Selection
- Click and drag in the PTY to select text
- Selection highlights the chosen region

### Keyboard Selection
- Hold `Shift` and use arrow keys to extend selection from the cursor

### Context Menu
- Right-click opens a context menu with selection actions
- Supported actions: Copy (only with an active selection), Paste
- Clicking outside the menu dismisses it
- Clipboard actions rely on OS tools (`pbcopy`/`pbpaste`, `clip`/`powershell`, or `xclip`) and warn if missing

### Bracketed Paste Mode

- Enabled on terminal startup for proper multi-line paste handling
- Pasted content is wrapped in bracketed paste sequences (`ESC[200~`...`ESC[201~`)
- Shell receives content as literal text, newlines are not executed during paste
- Disabled on terminal cleanup/exit

### Alt+Enter (Literal Newline)

| Input | Sequence | Behavior |
|-------|----------|----------|
| `Alt+Enter` | `ESC + \r` | Insert literal newline in shell (for multi-line command editing) |

## Rendering

### Process
1. Poll output from PTY reader thread
2. Feed output bytes to vt100 parser
3. Extract screen state from parser
4. Render cell-by-cell to ratatui frame buffer

### Color Mapping
vt100 colors are converted to ratatui colors:
- Named colors (black, red, green, etc.)
- 256-color palette indices
- RGB values

## Resize Handling

- Terminal resize events are detected in the main loop
- PTY is resized to match new dimensions
- vt100 parser is updated with new size

## Session Lifecycle

| Event | Action |
|-------|--------|
| Room selected (Enter) | Create session if not exists |
| Room deleted | Destroy session |
| Room renamed | Destroy session (working directory changed) |
| Application exit | All sessions terminated |

## Debug Logging

When `--debug-pty` flag is set:
- PTY output is logged to `~/.rooms/debug.log`
- Useful for diagnosing terminal rendering issues
