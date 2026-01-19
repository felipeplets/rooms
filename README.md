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

## Development

### Quick Start with GitHub Codespaces

[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/felipeplets/rooms)

This repository includes a complete development container configuration for GitHub Codespaces. Click the badge above to start coding immediately in a fully configured environment with:

- Rust toolchain (rustc, cargo, rustfmt, clippy)
- Bun runtime (for release scripts)
- GitHub Copilot integration
- Claude Code support
- All VS Code extensions for optimal Rust development

See [`.devcontainer/README.md`](.devcontainer/README.md) for more details.

### Local Development

```bash
# Clone the repository
git clone https://github.com/felipeplets/rooms.git
cd rooms

# Build the project
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Run linting
cargo clippy --all-targets --all-features -- -D warnings

# Run the application
cargo run
```

## Non-Goals

- Remote git operations (fetch/push/pull) - use the embedded shell
- GitHub API integration
- Cloud sync
- Windows support (initial release)

## License

MIT
