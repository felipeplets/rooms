# Overview

## Product Description

**Rooms** is a terminal user interface (TUI) application for managing Git worktrees. It provides a keyboard-driven interface with two panels: a sidebar listing all "rooms" (worktrees) and a main terminal panel for interacting with the selected room.

## Core Value Proposition

- Manage multiple Git worktrees from a single interface
- Instantly switch between worktrees with embedded terminal sessions
- Automate post-create setup commands per repository
- Local-only operation with no network dependencies

## Goals

1. **Simple worktree management**: Create, delete, and rename worktrees without memorizing git commands
2. **Integrated terminal**: Work directly in each worktree without leaving the application
3. **Automation**: Run configured setup commands automatically when creating new rooms
4. **Visibility**: Show room status, branch names, and operation progress clearly

## Non-Goals

1. **Git operations beyond worktrees**: No commit, push, pull, or merge functionality
2. **Remote repository management**: No GitHub/GitLab integration
3. **Multi-repository support**: Works within a single repository at a time
4. **Plugin system**: No extensibility beyond configuration

## Non-Negotiable Principles

### Privacy-First
- No telemetry collection
- No network calls
- All data stored locally in the repository

### Transparency
- Every action surfaces state transitions in the UI
- Local event log records all operations
- Status indicators show current room state

### Safety
- Confirmation dialogs for destructive actions
- Git branches are never deleted when removing rooms
- Dirty status warnings before deletion
- Atomic file writes prevent data corruption

### Performance
- UI remains responsive during background operations
- Non-blocking subprocess execution
- 50ms event polling for smooth interaction

### Quality
- Structured error messages with actionable information
- Consistent keyboard shortcuts across contexts
- Clear visual feedback for all operations
