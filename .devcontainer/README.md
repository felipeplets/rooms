# Development Container Configuration

This directory contains the configuration for GitHub Codespaces and VS Code Dev Containers.

## What's Included

### Base Image
- **Debian Bookworm** with Rust pre-installed
- Latest stable Rust toolchain with `rustfmt` and `clippy`

### Additional Tools
- **Git** - Version control (latest)
- **Node.js** - LTS version with npm
- **Bun** - Fast JavaScript runtime (for release scripts)

### VS Code Extensions

#### Rust Development
- `rust-analyzer` - Rust language server with intelligent code completion
- `even-better-toml` - TOML file support
- `crates` - Cargo.toml dependency management
- `vscode-lldb` - Debugger for Rust

#### AI Assistants
- `GitHub.copilot` - GitHub Copilot code suggestions
- `GitHub.copilot-chat` - GitHub Copilot chat interface
- `anthropic.claude-vscode` - Claude AI assistant

#### Development Tools
- `vscode-pull-request-github` - GitHub PR integration
- `gitlens` - Enhanced Git capabilities
- `EditorConfig` - Consistent code formatting
- `code-spell-checker` - Spell checking

### Configuration

The devcontainer includes optimized settings for:
- **Rust formatting** on save using `rustfmt`
- **Clippy linting** with warnings as errors
- **100-character line ruler** (matching project style)
- **Inlay hints** for better code understanding
- **GitHub Copilot** enabled for Rust, TOML, JSON, and YAML
- **Auto-completion** from both Copilot and rust-analyzer

## Usage

### GitHub Codespaces

1. Navigate to the repository on GitHub
2. Click the **Code** button
3. Select the **Codespaces** tab
4. Click **Create codespace on main** (or your branch)
5. Wait for the container to build and initialize
6. Start coding!

### VS Code Dev Containers (Local)

1. Install [Docker Desktop](https://www.docker.com/products/docker-desktop)
2. Install the [Remote - Containers](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers) extension
3. Open the repository in VS Code
4. Press `F1` and select **Dev Containers: Reopen in Container**
5. Wait for the container to build and initialize

## Post-Creation Setup

The `setup.sh` script runs automatically and:
- Installs Bun for running release scripts
- Configures Rust toolchain components
- Fetches cargo dependencies
- Displays environment information

## Quick Commands

Once inside the container:

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run clippy (linting)
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt

# Run the application
cargo run

# Test release script
bun scripts/release.ts --dry-run
```

## AI Assistant Tips

### GitHub Copilot
- Press `Ctrl+I` (or `Cmd+I` on macOS) for inline chat
- Use `/` commands in chat for specific actions
- Copilot provides context-aware code suggestions as you type

### Claude Code
- Access via the Anthropic extension
- Great for code reviews and explanations
- Can help refactor and improve code quality

## Troubleshooting

### Bun not found after setup
If `bun` command is not found, restart your terminal or source the bashrc:
```bash
source ~/.bashrc
```

### Rust toolchain issues
Verify the toolchain installation:
```bash
rustc --version
cargo --version
rustfmt --version
cargo clippy --version
```

### Extension not working
Some extensions may require a reload. Press `F1` and select **Developer: Reload Window**.

## Customization

To customize the devcontainer:
1. Edit `.devcontainer/devcontainer.json`
2. Rebuild the container: `F1` → **Dev Containers: Rebuild Container**

## Privacy & Security

This configuration:
- ✅ Does NOT include telemetry
- ✅ Respects the project's privacy-first approach
- ✅ All operations are local to the container
- ✅ Git config is mounted from your host machine

Note: AI assistants (Copilot/Claude) do communicate with their respective services for code suggestions, but this is opt-in and can be disabled in extension settings.
