// Input handling utilities.
//
// Most key handling is done directly in app.rs for now.
// This module can be expanded later for more complex input handling,
// such as text input for room creation prompts.

#![allow(dead_code)]

/// Represents the result of processing a key event.
#[derive(Debug, Clone, PartialEq)]
pub enum InputResult {
    /// Key was handled, continue running.
    Handled,

    /// Key was not handled.
    Ignored,

    /// Application should quit.
    Quit,
}
