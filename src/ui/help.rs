use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

/// Render the help overlay.
pub fn render_help(frame: &mut Frame, area: Rect) {
    // Center the help popup
    let popup_area = centered_rect(60, 70, area);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Sidebar",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  ?       ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle this help"),
        ]),
        Line::from(vec![
            Span::styled("  q       ", Style::default().fg(Color::Yellow)),
            Span::raw("Quit"),
        ]),
        Line::from(vec![
            Span::styled("  j/↓     ", Style::default().fg(Color::Yellow)),
            Span::raw("Move down"),
        ]),
        Line::from(vec![
            Span::styled("  k/↑     ", Style::default().fg(Color::Yellow)),
            Span::raw("Move up"),
        ]),
        Line::from(vec![
            Span::styled("  Enter   ", Style::default().fg(Color::Yellow)),
            Span::raw("Focus terminal"),
        ]),
        Line::from(vec![
            Span::styled("  a       ", Style::default().fg(Color::Yellow)),
            Span::raw("Add room (interactive)"),
        ]),
        Line::from(vec![
            Span::styled("  A       ", Style::default().fg(Color::Yellow)),
            Span::raw("Add room (quick)"),
        ]),
        Line::from(vec![
            Span::styled("  d       ", Style::default().fg(Color::Yellow)),
            Span::raw("Delete room"),
        ]),
        Line::from(vec![
            Span::styled("  D       ", Style::default().fg(Color::Yellow)),
            Span::raw("Delete room (no prompt)"),
        ]),
        Line::from(vec![
            Span::styled("  r       ", Style::default().fg(Color::Yellow)),
            Span::raw("Rename room"),
        ]),
        Line::from(vec![
            Span::styled("  R       ", Style::default().fg(Color::Yellow)),
            Span::raw("Refresh room list"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+b  ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle sidebar visibility"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Terminal",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  Ctrl+b  ", Style::default().fg(Color::Yellow)),
            Span::raw("Return to sidebar"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+t  ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle terminal visibility"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press ? or Esc to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, popup_area);
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
