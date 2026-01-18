# Keybindings

## Global Keys

Keys available regardless of context:

| Key | Action |
|-----|--------|
| `Ctrl+t` | Toggle terminal visibility |

## Sidebar Context

When sidebar is focused:

| Key | Action |
|-----|--------|
| `?` | Toggle help overlay |
| `Esc` | Close help overlay |
| `q` | Quit application |
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `Enter` | Focus terminal / start PTY session for selected room |
| `a` | Add room (interactive: prompts for name and branch) |
| `A` | Add room (quick: auto-generated name, current branch) |
| `Delete` | Delete room (shows confirmation dialog) |
| `Ctrl+Delete` | Delete room immediately (no confirmation dialog) |
| `r` | Rename room (prompts for new name) |
| `Ctrl+b` | Toggle sidebar visibility |

## Terminal Context (MainScene)

When terminal is focused:

| Key | Action |
|-----|--------|
| `Ctrl+b` | Focus sidebar (shows it if hidden) |
| `Ctrl+t` | Toggle terminal visibility |
| `PageUp` | Scroll up by one page in terminal history |
| `PageDown` | Scroll down by one page in terminal history |
| `Ctrl+U` | Scroll up by half page in terminal history |
| `Ctrl+D` | Scroll down by half page in terminal history |
| Mouse Scroll Up | Scroll up 3 lines in terminal history |
| Mouse Scroll Down | Scroll down 3 lines in terminal history |
| All other keys | Forwarded to shell/PTY |

### PTY Input Translation

Standard keys are translated to terminal escape sequences:

| Key | Sequence |
|-----|----------|
| `Enter` | `\r` |
| `Alt+Enter` | `ESC + \r` (literal newline in shell) |
| `Backspace` | `0x7f` |
| `Tab` | `\t` |
| Arrow keys | VT100 escape sequences |
| `F1`-`F12` | VT100 function key sequences |
| `Ctrl+<letter>` | ASCII codes 1-26 |

## Confirmation Dialog (Delete)

| Key | Action |
|-----|--------|
| `Tab` | Toggle between Cancel/Delete buttons |
| `h` / `←` | Select Cancel button |
| `l` / `→` | Select Delete button |
| `Enter` | Confirm selected action |
| `y` | Quick confirm (delete) |
| `n` | Quick cancel |
| `Esc` | Cancel |

## Text Input (Prompts)

| Key | Action |
|-----|--------|
| Characters | Insert at cursor |
| `Backspace` | Delete character before cursor |
| `Delete` | Delete character at cursor |
| `←` / `→` | Move cursor |
| `Home` | Move cursor to start |
| `End` | Move cursor to end |
| `Enter` | Confirm input |
| `Esc` | Cancel input |
