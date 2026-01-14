use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::app::{App, Focus};

/// Render the main scene panel (terminal area).
pub fn render_main_scene(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::MainScene;

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = if let Some(room) = app.selected_room() {
        format!(" {} ", room.name)
    } else {
        " Terminal ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    // Placeholder content until PTY is implemented
    let content = if let Some(room) = app.selected_room() {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("Room: {}", room.name),
                Style::default().fg(Color::White),
            )),
            Line::from(Span::styled(
                format!("Branch: {}", room.branch),
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                format!("Path: {}", room.path.display()),
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                format!("Status: {:?}", room.status),
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Terminal not yet implemented",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(Span::styled(
                "Press Esc to return to sidebar",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "No room selected",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Create a room with 'a' or 'A'",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}
