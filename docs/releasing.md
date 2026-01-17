# Releasing Rooms

This guide explains how to create a new release of `rooms`.

## Prerequisites

Before releasing, ensure you have:

1. **Repository Secrets Configured**
   - `HOMEBREW_TAP_TOKEN`: Fine-grained PAT for `felipeplets/homebrew-tap`
     - Go to GitHub → Settings → Developer settings → Personal access tokens → Fine-grained tokens
     - Create token with:
       - Resource owner: `felipeplets`
       - Repository access: Only `homebrew-tap`
       - Permissions: Contents (Read and write)
     - Add to repository secrets in `felipeplets/rooms`

2. **Conventional Commits**
   - All commits since last release follow conventional commit format
   - PR titles (which become squash commit messages) must be conventional

## Creating a Release

### Step 1: Verify Commits

Check that commits since the last tag follow conventional format:

```bash
# View last tag
git describe --tags --abbrev=0

# View commits since last tag
git log $(git describe --tags --abbrev=0)..HEAD --oneline
```

### Step 2: Run Dry Run (Recommended)

1. Go to **Actions** → **Release** workflow
2. Click **Run workflow**
3. Enable **Dry run** checkbox
4. Click **Run workflow**
5. Review the output:
   - Detected version bump
   - Generated changelog
   - All CI checks pass

### Step 3: Create Release

1. Go to **Actions** → **Release** workflow
2. Click **Run workflow**
3. Leave **Dry run** unchecked
4. Click **Run workflow**

The workflow will:
- Run all CI checks (format, clippy, test, build)
- Detect version from commits
- Update `Cargo.toml`
- Create commit and tag
- Push to `main`
- Create GitHub Release
- Trigger Homebrew tap update

### Step 4: Verify

After the workflow completes:

1. Check [GitHub Releases](https://github.com/felipeplets/rooms/releases) for new release
2. Verify changelog content
3. Check [homebrew-tap](https://github.com/felipeplets/homebrew-tap) for formula update

## Version Override

To force a specific version (useful for first release or corrections):

1. Run workflow
2. Enter version in **Force version** field (e.g., `1.0.0`)
3. This overrides automatic detection

## Troubleshooting

### "No conventional commits found"

**Cause:** Commits since last tag don't follow conventional format.

**Solutions:**
- Use `--force-version=X.Y.Z` to override
- Ensure future commits follow format

### Homebrew tap not updated

**Cause:** `HOMEBREW_TAP_TOKEN` not set or invalid.

**Solutions:**
1. Verify token exists in repository secrets
2. Check token hasn't expired
3. Verify token has correct permissions
4. Check homebrew-tap repository for failed workflow

### CI checks failing

**Cause:** Code doesn't pass format, clippy, or tests.

**Solutions:**
1. Run locally: `cargo fmt --check && cargo clippy && cargo test`
2. Fix issues
3. Commit with fix commit
4. Re-run release

### Release created but wrong version

**Cause:** Unexpected commits or version detection issue.

**Solutions:**
1. Delete the tag and release manually
2. Re-run with `--force-version=X.Y.Z`

## Commit Message Examples

```bash
# Feature (minor bump)
git commit -m "feat(room): add room rename capability"

# Fix (patch bump)
git commit -m "fix(terminal): handle resize correctly"

# Breaking change (major bump)
git commit -m "feat(config)!: change config file format"

# With PR reference (added automatically by GitHub)
# PR title: "feat(room): add room rename capability"
# Squash commit: "feat(room): add room rename capability (#123)"
```

## Local Testing

Test the release script locally (requires [Bun](https://bun.sh)):

```bash
# Dry run
bun scripts/release.ts --dry-run

# Force version
bun scripts/release.ts --dry-run --force-version=1.0.0
```

## Security Notes

- Never commit secrets or tokens
- Use fine-grained PATs with minimal scope
- Release workflow only runs on manual trigger
- Uses GitHub-owned actions (`actions/*`) plus `oven-sh/setup-bun` for Bun runtime
