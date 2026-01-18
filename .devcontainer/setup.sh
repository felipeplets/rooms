#!/bin/bash
set -e

echo "🚀 Setting up rooms development environment..."

# Install Bun (for release scripts)
# Using official Bun installer from https://bun.sh/install
# This is the recommended installation method by the Bun team
echo "📦 Installing Bun..."
curl -fsSL https://bun.sh/install | bash
export BUN_INSTALL="$HOME/.bun"
export PATH="$BUN_INSTALL/bin:$PATH"

# Add Bun to shell profile for persistence
echo 'export BUN_INSTALL="$HOME/.bun"' >> ~/.bashrc
echo 'export PATH="$BUN_INSTALL/bin:$PATH"' >> ~/.bashrc

# Verify Bun installation
if command -v bun &> /dev/null; then
    echo "✅ Bun installed: $(bun --version)"
else
    echo "⚠️  Bun installation may require a new shell session"
fi

# Install Rust components (if not already present)
echo "🦀 Setting up Rust toolchain..."
set +e
rustup component add rustfmt clippy 2>/dev/null
rustup_status=$?
set -e
if [ "$rustup_status" -ne 0 ]; then
    echo "⚠️  Failed to add Rust components (rustfmt, clippy). Continuing without them."
fi

# Verify Rust installation
echo "📋 Rust toolchain information:"
rustc --version
cargo --version
rustfmt --version
cargo clippy --version

# Install cargo dependencies
echo "📚 Fetching cargo dependencies..."
cargo fetch

# Verify git is available
echo "🔧 Git version: $(git --version)"

echo ""
echo "✅ Development environment setup complete!"
echo ""
echo "🎯 Quick start:"
echo "  - Run 'cargo build' to build the project"
echo "  - Run 'cargo test' to run tests"
echo "  - Run 'cargo run' to start the TUI"
echo "  - Run 'bun scripts/release.ts --dry-run' to test release process"
echo ""
echo "🤖 AI tools available:"
echo "  - GitHub Copilot (Ctrl+I for inline chat)"
echo "  - Claude Code (via Anthropic extension)"
echo ""
