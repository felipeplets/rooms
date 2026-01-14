# rooms

Terminal UI for managing Git worktrees.

## Overview

`rooms` is a keyboard-driven TUI that helps you create and manage Git worktrees. Each worktree becomes a "room" with its own embedded shell, letting you work on multiple branches simultaneously without stashing or switching.

## Features

- **Two-panel layout**: Sidebar listing rooms + embedded terminal for the selected room
- **Keyboard-driven**: Every action has a shortcut
- **Safe by default**: Confirms destructive actions, never deletes branches automatically
- **Transparent**: Every operation shows its status in the UI and logs

## Privacy

**No telemetry. No network calls. Everything stays local.**

- All data stored in `<repo>/.rooms/`
- No external services contacted
- No usage data collected

## Installation

### From source

```bash
cargo install --path .
```

### Homebrew (coming soon)

```bash
brew install felipeplets/tap/rooms
```

## Usage

```bash
# Launch the TUI in a git repository
cd your-repo
rooms

# Show version
rooms --version

# Show help
rooms --help
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `?` | Help overlay |
| `q` | Quit |
| `a` | Add room (interactive) |
| `A` | Add room (silent/quick) |
| `d` | Delete room |
| `j/k` | Navigate rooms |
| `Enter` | Focus terminal |
| `Esc` | Return to sidebar |
| `Ctrl+b` | Toggle sidebar |
| `Ctrl+t` | Toggle terminal |

## Non-Goals

- Remote git operations (fetch/push/pull) - use the embedded shell
- GitHub API integration
- Cloud sync
- Windows support (initial release)

## License

MIT
