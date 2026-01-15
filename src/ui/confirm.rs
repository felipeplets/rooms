use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::room::DirtyStatus;

/// State for a confirmation dialog.
#[derive(Debug, Clone, Default)]
pub enum ConfirmState {
    /// No confirmation dialog active.
    #[default]
    None,

    /// Confirming room deletion.
    DeleteRoom {
        room_name: String,
        room_path: String,
        branch: String,
        dirty_status: Option<DirtyStatus>,
        /// Current selection: true = confirm (delete), false = cancel
        selected_confirm: bool,
    },
}

impl ConfirmState {
    /// Start a room deletion confirmation.
    pub fn start_delete(
        room_name: String,
        room_path: String,
        branch: String,
        dirty_status: Option<DirtyStatus>,
    ) -> Self {
        Self::DeleteRoom {
            room_name,
            room_path,
            branch,
            dirty_status,
            selected_confirm: false, // Default to cancel for safety
        }
    }

    /// Check if a confirmation dialog is active.
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Toggle the selection between confirm and cancel.
    pub fn toggle_selection(&mut self) {
        if let Self::DeleteRoom {
            selected_confirm, ..
        } = self
        {
            *selected_confirm = !*selected_confirm;
        }
    }

    /// Confirm the action. Returns the room name if confirmed, None if cancelled.
    pub fn confirm(&mut self) -> Option<String> {
        match std::mem::take(self) {
            Self::DeleteRoom {
                room_name,
                selected_confirm: true,
                ..
            } => Some(room_name),
            Self::DeleteRoom {
                selected_confirm: false,
                ..
            } => None,
            Self::None => None,
        }
    }

    /// Cancel the confirmation dialog.
    pub fn cancel(&mut self) {
        *self = Self::None;
    }
}

/// Render the confirmation dialog overlay.
pub fn render_confirm(frame: &mut Frame, area: Rect, confirm: &ConfirmState) {
    let (title, room_name, room_path, branch, dirty_status, selected_confirm) = match confirm {
        ConfirmState::None => return,
        ConfirmState::DeleteRoom {
            room_name,
            room_path,
            branch,
            dirty_status,
            selected_confirm,
        } => (
            "Delete Room",
            room_name,
            room_path,
            branch,
            dirty_status,
            *selected_confirm,
        ),
    };

    // Center the dialog
    let popup_area = centered_rect(60, 50, area);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Build content lines
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Room: ", Style::default().fg(Color::Gray)),
            Span::styled(room_name.as_str(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Path: ", Style::default().fg(Color::Gray)),
            Span::styled(room_path.as_str(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Branch: ", Style::default().fg(Color::Gray)),
            Span::styled(branch.as_str(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
    ];

    // Add dirty warning if applicable
    if let Some(status) = dirty_status {
        if status.is_dirty {
            lines.push(Line::from(vec![Span::styled(
                "WARNING: Uncommitted changes detected!",
                Style::default().fg(Color::Yellow),
            )]));
            lines.push(Line::from(vec![
                Span::styled("  Modified: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    status.modified_count.to_string(),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled("  Untracked: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    status.untracked_count.to_string(),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
            if !status.summary.is_empty() {
                lines.push(Line::from(""));
                for file_line in status.summary.lines().take(3) {
                    lines.push(Line::from(vec![Span::styled(
                        format!("  {}", file_line),
                        Style::default().fg(Color::DarkGray),
                    )]));
                }
            }
            lines.push(Line::from(""));
        }
    }

    lines.push(Line::from(vec![Span::styled(
        "Note: Branch will NOT be deleted.",
        Style::default().fg(Color::Gray),
    )]));
    lines.push(Line::from(""));
    lines.push(Line::from("Are you sure you want to delete this room?"));
    lines.push(Line::from(""));

    // Layout: content and buttons
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(inner);

    // Content
    let content = Paragraph::new(lines).alignment(Alignment::Left);
    frame.render_widget(content, chunks[0]);

    // Buttons
    let button_cancel = if selected_confirm {
        Span::styled("  Cancel  ", Style::default().fg(Color::Gray))
    } else {
        Span::styled(
            "[ Cancel ]",
            Style::default().fg(Color::White).bg(Color::DarkGray),
        )
    };

    let button_delete = if selected_confirm {
        Span::styled(
            "[ Delete ]",
            Style::default().fg(Color::White).bg(Color::Red),
        )
    } else {
        Span::styled("  Delete  ", Style::default().fg(Color::Red))
    };

    let buttons = Paragraph::new(Line::from(vec![
        button_cancel,
        Span::raw("     "),
        button_delete,
    ]))
    .alignment(Alignment::Center);

    frame.render_widget(buttons, chunks[1]);
}

/// Create a centered rectangle with the given percentage width and height.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confirm_state_toggle() {
        let mut state = ConfirmState::start_delete(
            "test-room".to_string(),
            "/path/to/room".to_string(),
            "test-branch".to_string(),
            None,
        );

        // Default to cancel (false)
        if let ConfirmState::DeleteRoom {
            selected_confirm, ..
        } = &state
        {
            assert!(!selected_confirm);
        }

        state.toggle_selection();

        if let ConfirmState::DeleteRoom {
            selected_confirm, ..
        } = &state
        {
            assert!(selected_confirm);
        }
    }

    #[test]
    fn test_confirm_state_confirm_selected() {
        let mut state = ConfirmState::start_delete(
            "test-room".to_string(),
            "/path/to/room".to_string(),
            "test-branch".to_string(),
            None,
        );

        state.toggle_selection(); // Select confirm
        let result = state.confirm();

        assert_eq!(result, Some("test-room".to_string()));
        assert!(!state.is_active());
    }

    #[test]
    fn test_confirm_state_cancel_selected() {
        let mut state = ConfirmState::start_delete(
            "test-room".to_string(),
            "/path/to/room".to_string(),
            "test-branch".to_string(),
            None,
        );

        // Don't toggle, keep cancel selected
        let result = state.confirm();

        assert_eq!(result, None);
        assert!(!state.is_active());
    }

    #[test]
    fn test_confirm_state_cancel_method() {
        let mut state = ConfirmState::start_delete(
            "test-room".to_string(),
            "/path/to/room".to_string(),
            "test-branch".to_string(),
            None,
        );

        state.cancel();

        assert!(!state.is_active());
    }
}
