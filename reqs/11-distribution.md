# Distribution

## Installation Methods

### Cargo (crates.io)

```bash
cargo install rooms
```

### Homebrew

```bash
brew tap felipeplets/rooms
brew install rooms
```

### Binary Releases

Pre-built binaries available on GitHub Releases for:
- macOS (Intel and Apple Silicon)
- Linux (x86_64)

## Build Requirements

- Rust 1.70+
- Standard system libraries for PTY support

## Dependencies

Key runtime dependencies:
- `ratatui`: Terminal UI framework
- `portable-pty`: Cross-platform PTY handling
- `vt100`: Terminal emulation/parsing
- `crossterm`: Terminal input/output
- `serde` / `toml`: Configuration parsing
- `uuid`: Room identifier generation

## Homebrew Formula

Maintained in `felipeplets/homebrew-rooms` repository.

Auto-update workflow:
- Triggered on new GitHub releases
- Updates formula with new version and SHA256
- Creates pull request automatically

## Versioning

Follows Semantic Versioning (SemVer):
- MAJOR: Breaking changes
- MINOR: New features, backwards compatible
- PATCH: Bug fixes

## Platform Support

| Platform | Status |
|----------|--------|
| macOS | Supported |
| Linux | Supported |
| Windows | Not currently supported |
