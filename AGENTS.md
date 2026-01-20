# Rooms — Implementation Instructions (Claude)

You are implementing `rooms`, a Rust + ratatui terminal UI to manage Git worktrees ("rooms").
Repository: https://github.com/felipeplets/rooms

## Worktree Context

**CRITICAL**: Always work within the current worktree directory where you were initiated. This project uses git worktrees for development, and each worktree is an independent working copy:

- Check your current working directory at the start of any session
- All file edits, reads, and operations must use paths within the current worktree
- Never modify files in the parent repository or other worktrees
- The current worktree path pattern is: `.rooms/<worktree-name>/`

## Non-negotiable rules
1. Privacy-first: DO NOT add telemetry. DO NOT add network calls. Everything is local-only.
2. Transparency: Every action must surface state transitions in the UI and a local event log.
3. Safety: Confirm destructive actions. Never delete branches by default. Never shell out to dangerous commands like `rm -rf`.
4. Performance: Keep UI responsive; do not block the render loop on subprocess execution.
5. Quality: Prefer small PRs. Add tests where feasible (especially for name generation, config/state parsing, git command wrappers).

## Product intent
- Two panels: sidebar list of rooms + main embedded terminal for selected room.
- Keyboard-driven UX. Panels can be hidden.
- Create room (interactive and silent), remove room (safe), run post-create commands per room.

## Tech constraints
- Rust + ratatui.
- Use git worktree (`git worktree list/add/remove`) as source of truth.
- Persist local settings/configurations

## Coding guidelines
- Wrap all git/subprocess calls in a single module with structured results (stdout/stderr/exit code).
- All state writes must be atomic (write temp then rename).
- Errors must include actionable messages.
- Used SOLID principles to structure the code

## Requirements
- Follow requirements described in ./reqs/ and point for any requirement inconsistency
- Always update requirements based on new feature requests, changes or fixes.

## Commit Message Convention

Uses [Conventional Commits](https://conventionalcommits.org/) for automated releases.

### Format
```
<type>[scope][!]: <description>
```

### Types
- `feat`: New feature → minor version bump
- `fix`: Bug fix → patch version bump
- `docs`: Documentation only → patch
- `chore`: Maintenance → patch
- `refactor`, `test`, `perf`, `style`, `ci`, `build`: → patch

### Breaking Changes
- Add `!` after type: `feat!: breaking change`
- Or `BREAKING CHANGE:` in footer → major version bump

### Scope (optional)
Use module names: `git`, `room`, `ui`, `terminal`, `config`, `state`

### PR Titles
PRs are squash-merged; PR title becomes the commit message.
PR titles MUST follow conventional commit format.

## Before committing code
- Check that format is correct using `cargo fmt --check`
- run `cargo clippy --all-targets --all-features -- -D warnings`
- run `cargo build --verbose`
- run `cargo test --verbose`

# Pull Request creation
You should either create the branch, commit the code and open a Pull request or give the summary of commands to do it to the user. 