# Release Process

## Overview

Automated release pipeline using conventional commits for versioning and changelog generation.

## Triggering a Release

Releases are triggered **manually only** via GitHub Actions `workflow_dispatch`:

1. Navigate to Actions → Release workflow
2. Click "Run workflow"
3. Optionally enable `dry_run` to preview changes
4. Optionally set `force_version` to override auto-detection

## Versioning Rules

Follows [Semantic Versioning](https://semver.org/):

| Change Type | Version Bump | Example |
|-------------|--------------|---------|
| Breaking change (`!` or `BREAKING CHANGE`) | Major | 1.0.0 → 2.0.0 |
| New feature (`feat`) | Minor | 1.0.0 → 1.1.0 |
| Bug fix, docs, chore, etc. | Patch | 1.0.0 → 1.0.1 |

## Conventional Commits

All commits must follow [Conventional Commits](https://conventionalcommits.org/) format:

```
<type>[scope][!]: <description>
```

### Commit Types

| Type | Description | Bump |
|------|-------------|------|
| `feat` | New feature | minor |
| `fix` | Bug fix | patch |
| `docs` | Documentation | patch |
| `style` | Code style (formatting) | patch |
| `refactor` | Code refactoring | patch |
| `perf` | Performance improvement | patch |
| `test` | Adding/fixing tests | patch |
| `build` | Build system changes | patch |
| `ci` | CI configuration | patch |
| `chore` | Maintenance tasks | patch |
| `revert` | Revert previous commit | patch |

### Scopes

Optional scopes based on module names:
- `git` - Git operations
- `room` - Room management
- `ui` - User interface
- `terminal` - Terminal/PTY handling
- `config` - Configuration
- `state` - State management

### Breaking Changes

Indicate breaking changes by:
- Adding `!` after type/scope: `feat(room)!: change API`
- Including `BREAKING CHANGE:` in commit footer

## Release Workflow Steps

1. **CI Checks** - Format, clippy, build, test
2. **Version Detection** - Parse commits since last tag
3. **Changelog Generation** - Create Nx-style changelog
4. **Version Bump** - Update `Cargo.toml`
5. **Git Operations** - Commit, tag, push
6. **GitHub Release** - Create release with changelog
7. **Homebrew Update** - Dispatch to homebrew-tap

## Changelog Format

Generated changelog follows Nx-style formatting:

```markdown
## 0.2.0 (2024-01-16)

### 🚨 Breaking Changes
- **keybindings:** change delete key from d to Delete (#4)

### 🚀 Features
- **room:** add room rename capability (#2)

### 🩹 Fixes
- **tests:** formatting and tests across codebase (#3)
```

## Homebrew Integration

After release:
1. `repository_dispatch` sent to `felipeplets/homebrew-tap`
2. Homebrew tap workflow downloads tarball
3. Computes SHA256
4. Updates `Formula/rooms.rb`
5. Commits and pushes

## Required Secrets

| Secret | Purpose | Repository |
|--------|---------|------------|
| `GITHUB_TOKEN` | Releases, commits | Built-in |
| `HOMEBREW_TAP_TOKEN` | Cross-repo dispatch | Fine-grained PAT |

### HOMEBREW_TAP_TOKEN Requirements

- Fine-grained personal access token
- Scoped to `felipeplets/homebrew-tap` only
- Permission: `Contents: Read and write`

## Security Considerations

- Minimal third-party GitHub Actions (`actions/*` and `oven-sh/setup-bun`)
- Fine-grained PAT scoped to single repository
- Release script runs with Bun (no npm dependencies)
- All operations logged in GitHub Actions
- Dry-run mode for safe testing

## Dry Run Mode

Enable `dry_run` to:
- Execute all checks and version detection
- Generate changelog preview
- Skip all Git operations (commit, tag, push)
- Skip GitHub Release creation
- Skip Homebrew dispatch

## Manual Override

Use `force_version` input to:
- Override auto-detected version
- Useful for initial releases or corrections
- Format: `X.Y.Z` (no `v` prefix)
