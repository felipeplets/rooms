use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

use crate::state::RoomStatus;

use super::app::{App, Focus};

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

            let content = Line::from(vec![
                Span::styled(
                    format!("{} ", status_icon),
                    Style::default().fg(status_color),
                ),
                Span::styled(&room.name, style),
            ]);

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
