use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::state::RoomStatus;

use super::app::{App, Focus};

/// Truncate a string to fit within max_width, adding ellipsis if needed.
/// Uses unicode width to handle multi-byte characters correctly.
fn truncate_with_ellipsis(text: &str, max_width: usize) -> String {
    let text_width = text.width();
    if text_width <= max_width {
        return text.to_string();
    }

    // Reserve 1 character for the ellipsis
    if max_width <= 1 {
        return "…".to_string();
    }

    let target_width = max_width - 1;
    let mut result = String::new();
    let mut current_width = 0;

    for ch in text.chars() {
        let ch_width = ch.width().unwrap_or(0);
        if current_width + ch_width > target_width {
            break;
        }
        result.push(ch);
        current_width += ch_width;
    }

    result.push('…');
    result
}

/// Render the sidebar panel showing the list of rooms.
pub fn render_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::Sidebar;

    // Build border style based on focus
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" Rooms ")
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.state.rooms.is_empty() {
        // Show empty state
        let empty_msg = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No rooms yet",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press 'a' to create one",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let paragraph = ratatui::widgets::Paragraph::new(empty_msg)
            .block(block)
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(paragraph, area);
        return;
    }

    // Calculate available width for text (area width minus borders)
    let available_width = area.width.saturating_sub(2) as usize;

    // Status icon + space takes 2 characters
    const STATUS_PREFIX_WIDTH: usize = 2;
    // Branch indicator "  └─ " takes 5 characters
    const BRANCH_PREFIX_WIDTH: usize = 5;

    let room_name_max_width = available_width.saturating_sub(STATUS_PREFIX_WIDTH);
    let branch_name_max_width = available_width.saturating_sub(BRANCH_PREFIX_WIDTH);

    // Build list items
    let items: Vec<ListItem> = app
        .state
        .rooms
        .iter()
        .enumerate()
        .map(|(i, room)| {
            let status_icon = status_icon(&room.status);
            let status_color = status_color(&room.status);

            let is_selected = i == app.selected_index;
            let style = if is_selected && is_focused {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().fg(Color::Black).bg(Color::DarkGray)
            } else {
                Style::default()
            };

            // Truncate room name and branch if they exceed available width
            let room_name = truncate_with_ellipsis(&room.name, room_name_max_width);
            let branch_name = truncate_with_ellipsis(&room.branch, branch_name_max_width);

            let content = vec![
                // Line 1: Status icon + Room name
                Line::from(vec![
                    Span::styled(
                        format!("{} ", status_icon),
                        Style::default().fg(status_color),
                    ),
                    Span::styled(room_name, style),
                ]),
                // Line 2: Branch indicator + Branch name
                Line::from(vec![
                    Span::styled("  └─ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(branch_name, Style::default().fg(Color::DarkGray)),
                ]),
                // Line 3: Empty line for spacing
                Line::from(""),
            ];

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    // We need to track selection state
    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_index));

    frame.render_stateful_widget(list, area, &mut list_state);
}

/// Get the status icon for a room status.
fn status_icon(status: &RoomStatus) -> &'static str {
    match status {
        RoomStatus::Idle => "○",
        RoomStatus::Creating => "◐",
        RoomStatus::PostCreateRunning => "◐",
        RoomStatus::Ready => "●",
        RoomStatus::Error => "!",
        RoomStatus::Deleting => "◐",
        RoomStatus::Orphaned => "?",
    }
}

/// Get the color for a room status.
fn status_color(status: &RoomStatus) -> Color {
    match status {
        RoomStatus::Idle => Color::White,
        RoomStatus::Creating => Color::Yellow,
        RoomStatus::PostCreateRunning => Color::Yellow,
        RoomStatus::Ready => Color::Green,
        RoomStatus::Error => Color::Red,
        RoomStatus::Deleting => Color::Yellow,
        RoomStatus::Orphaned => Color::DarkGray,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_with_ellipsis_no_truncation_needed() {
        assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
        assert_eq!(truncate_with_ellipsis("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_with_ellipsis_truncates() {
        assert_eq!(truncate_with_ellipsis("hello world", 8), "hello w…");
        assert_eq!(truncate_with_ellipsis("hello world", 6), "hello…");
    }

    #[test]
    fn test_truncate_with_ellipsis_edge_cases() {
        assert_eq!(truncate_with_ellipsis("hello", 1), "…");
        assert_eq!(truncate_with_ellipsis("hello", 0), "…");
        assert_eq!(truncate_with_ellipsis("", 5), "");
    }

    #[test]
    fn test_truncate_with_ellipsis_unicode() {
        // Test with multi-byte characters
        assert_eq!(truncate_with_ellipsis("日本語", 4), "日…");
        assert_eq!(truncate_with_ellipsis("日本語", 6), "日本語");
    }
}
