# Glossary

## Room
A managed Git worktree with associated metadata (name, status, timestamps). Rooms are the primary unit of organization in the application.

## Worktree
A Git worktree—a linked working directory that shares the repository's Git data but has its own working tree and index. Created via `git worktree add`.

## Sidebar
The left panel displaying the list of rooms with their status icons and branch names. Supports navigation and room management operations.

## MainScene
The right panel containing the embedded terminal for the selected room. Displays the PTY session output and accepts keyboard input.

## PTY (Pseudo-Terminal)
A virtual terminal device pair that emulates a hardware terminal. Used to run shell sessions within the application.

## vt100
A terminal emulation standard. The application uses a vt100 parser to interpret ANSI escape sequences from shell output.

## Post-Create Commands
Configured shell commands that run automatically after a room is created. Used for setup tasks like `npm install` or `make setup`.

## Dirty Status
Indicates a room has uncommitted changes (modified or untracked files). Checked before deletion to warn users.

## Orphaned
A room status indicating the worktree directory no longer exists on disk. Detected during path validation on startup.

## Focus
Which panel currently receives keyboard input. Either `Sidebar` or `MainScene`.

## Status Icons
Visual indicators showing room state:
- `○` Idle
- `◐` Creating
- `●` Ready
- `!` Error
- `?` Orphaned
