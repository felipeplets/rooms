# Rooms — Implementation Instructions (Claude)

You are implementing `rooms`, a Rust + ratatui terminal UI to manage Git worktrees ("rooms").
Repository: https://github.com/felipeplets/rooms

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