# Non-Functional Requirements

## Privacy

### REQ-NF-PRIV-1: No Telemetry
The application MUST NOT collect or transmit any usage data, analytics, or telemetry.

### REQ-NF-PRIV-2: No Network Calls
The application MUST NOT make any network requests. All operations are local-only.

### REQ-NF-PRIV-3: Local Data Storage
All configuration and logs MUST be stored within the repository directory. Configuration lives at
`{primary_worktree_root}/.roomsrc.json` and logs under `.rooms/`. No persistent room state file is
stored.

## Safety

### REQ-NF-SAFE-1: Destructive Action Confirmation
Destructive operations (delete room) MUST require explicit user confirmation via a dialog.

### REQ-NF-SAFE-2: Branch Preservation
Deleting a room MUST NOT delete the associated Git branch. Only the worktree is removed.

### REQ-NF-SAFE-3: Dirty Status Warning
Before deleting a room with uncommitted changes, the application MUST display a warning showing the modified/untracked files.

### REQ-NF-SAFE-4: No Dangerous Commands
The application MUST NOT execute dangerous shell commands like `rm -rf`. Worktree removal uses Git's built-in command.

## Performance

### REQ-NF-PERF-1: Non-Blocking UI
The UI render loop MUST NOT block on subprocess execution. Long-running operations run in background threads.

### REQ-NF-PERF-2: Event Polling
Input events MUST be polled with a maximum 50ms timeout to ensure responsive interaction.

### REQ-NF-PERF-3: Background Operations
Room creation and PTY output reading MUST run in separate threads.

## Reliability

### REQ-NF-REL-1: Orphaned Room Detection
Prunable worktrees reported by Git MUST be surfaced as Orphaned/Failed entries in the UI.

### REQ-NF-REL-2: Error Messages
All errors MUST include actionable messages describing what went wrong and potential remediation.

### REQ-NF-REL-3: State Recovery
The application MUST handle missing or corrupted transient status by falling back to the Git worktree list.
