use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::room::{RoomInfo, RoomStatus};

use super::app::{App, Focus, RoomSection};

const PRUNABLE_LABEL: &str = " [prunable]";
const ERROR_LABEL: &str = " [error]";

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
    const ITEM_PADDING: u16 = 1;

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

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.rooms.is_empty() {
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
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(paragraph, inner);
        return;
    }

    // Calculate available width for text (area width minus borders)
    let available_width = inner.width as usize;
    let content_width = available_width.saturating_sub((ITEM_PADDING * 2) as usize);

    // Status icon + space takes 2 characters
    const STATUS_PREFIX_WIDTH: usize = 2;
    // Branch indicator "  └─ " takes 5 characters
    const BRANCH_PREFIX_WIDTH: usize = 5;
    const PRIMARY_LABEL: &str = " [primary]";

    let left_pad = " ".repeat(ITEM_PADDING as usize);
    let right_pad = " ".repeat(ITEM_PADDING as usize);

    let mut items: Vec<ListItem> = Vec::new();
    let mut list_state = ListState::default();
    let mut list_index = 0;
    let mut selected_list_index = None;
    let mut current_section: Option<RoomSection> = None;
    let mut has_rendered_section = false;

    for (i, room) in app.rooms.iter().enumerate() {
        let section = app.room_section(room);
        if current_section != Some(section) {
            current_section = Some(section);
            if has_rendered_section {
                items.push(ListItem::new(Line::from("")));
                list_index += 1;
            }
            items.push(ListItem::new(Line::from(vec![
                Span::raw(left_pad.clone()),
                Span::styled(
                    section_title(section),
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                ),
            ])));
            items.push(ListItem::new(Line::from("")));
            list_index += 1;
            list_index += 1;
            has_rendered_section = true;
        }

        let status_icon = status_icon_for_room(room, section);
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

        let failed_label = failed_reason_label(room);
        let primary_label = if room.is_primary { PRIMARY_LABEL } else { "" };
        let label_width = primary_label.width() + failed_label.width();
        let room_name_max_width = content_width.saturating_sub(STATUS_PREFIX_WIDTH + label_width);
        let branch_name_max_width = content_width.saturating_sub(BRANCH_PREFIX_WIDTH);

        let room_name = truncate_with_ellipsis(&room.name, room_name_max_width);
        let branch = room.branch.as_deref().unwrap_or("detached");
        let branch_name = truncate_with_ellipsis(branch, branch_name_max_width);

        let mut title_spans = vec![
            Span::raw(left_pad.clone()),
            Span::styled(
                format!("{} ", status_icon),
                Style::default().fg(status_color),
            ),
            Span::styled(room_name, style),
        ];
        if !failed_label.is_empty() {
            title_spans.push(Span::styled(
                failed_label,
                Style::default().fg(Color::LightRed),
            ));
        }
        if room.is_primary {
            title_spans.push(Span::styled(
                primary_label,
                Style::default().fg(Color::Yellow),
            ));
        }
        title_spans.push(Span::raw(right_pad.clone()));

        let content = vec![
            // Line 1: Status icon + Room name + primary label
            Line::from(title_spans),
            // Line 2: Branch indicator + Branch name
            Line::from(vec![
                Span::raw(left_pad.clone()),
                Span::styled("  └─ ", Style::default().fg(Color::DarkGray)),
                Span::styled(branch_name, Style::default().fg(Color::DarkGray)),
                Span::raw(right_pad.clone()),
            ]),
        ];

        items.push(ListItem::new(content).style(style));
        if is_selected {
            selected_list_index = Some(list_index);
        }
        list_index += 1;
    }

    let list = List::new(items).highlight_style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    list_state.select(selected_list_index);

    frame.render_stateful_widget(list, inner, &mut list_state);
}

fn section_title(section: RoomSection) -> &'static str {
    match section {
        RoomSection::Active => "ACTIVE",
        RoomSection::Inactive => "INACTIVE",
        RoomSection::Failed => "FAILED",
    }
}

fn status_icon_for_room(room: &RoomInfo, section: RoomSection) -> &'static str {
    if section == RoomSection::Inactive && room.status == RoomStatus::Ready {
        return "○";
    }

    match room.status {
        RoomStatus::Idle => "○",
        RoomStatus::Creating => "◐",
        RoomStatus::PostCreateRunning => "◐",
        RoomStatus::Ready => "●",
        RoomStatus::Error => "!",
        RoomStatus::Deleting => "◐",
        RoomStatus::Orphaned => "?",
    }
}

fn failed_reason_label(room: &RoomInfo) -> &'static str {
    if !matches!(room.status, RoomStatus::Error | RoomStatus::Orphaned) && !room.is_prunable {
        return "";
    }

    if room.is_prunable {
        PRUNABLE_LABEL
    } else if room.last_error.is_some() {
        ERROR_LABEL
    } else {
        ""
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

    fn make_room(name: &str, status: RoomStatus) -> RoomInfo {
        RoomInfo {
            name: name.to_string(),
            branch: Some("main".to_string()),
            path: std::path::PathBuf::from("/tmp"),
            status,
            is_prunable: false,
            last_error: None,
            is_primary: false,
        }
    }

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

    #[test]
    fn test_status_icon_inactive_ready() {
        let room = make_room("inactive", RoomStatus::Ready);
        let icon = status_icon_for_room(&room, RoomSection::Inactive);
        assert_eq!(icon, "○");
    }

    #[test]
    fn test_failed_reason_label_prunable() {
        let mut room = make_room("failed", RoomStatus::Orphaned);
        room.is_prunable = true;
        let label = failed_reason_label(&room);
        assert_eq!(label, PRUNABLE_LABEL);
    }

    #[test]
    fn test_failed_reason_label_error() {
        let mut room = make_room("failed", RoomStatus::Error);
        room.last_error = Some("boom".to_string());
        let label = failed_reason_label(&room);
        assert_eq!(label, ERROR_LABEL);
    }
}
