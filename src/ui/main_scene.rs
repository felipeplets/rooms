use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::app::{App, Focus, PendingRoomStatus, RoomSection};
use crate::terminal::debug_log;

// UI message constants
const PRUNABLE_WORKTREE_MESSAGE: &str = "Worktree is prunable - Press Enter to prune";
const FAILED_WORKTREE_DEFAULT_MESSAGE: &str =
    "Worktree is in a failed state. Check logs for details.";

/// Convert vt100 color to ratatui Color.
fn vt100_color_to_ratatui(color: vt100::Color, is_foreground: bool) -> Color {
    match color {
        vt100::Color::Default => {
            if is_foreground {
                // Currently no distinction between foreground and background defaults.
            }
            Color::Reset
        }
        vt100::Color::Idx(idx) => indexed_to_color(idx),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

/// Convert 256-color palette index to ratatui Color.
fn indexed_to_color(idx: u8) -> Color {
    match idx {
        // Standard ANSI colors (0-7)
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::White,
        // Bright ANSI colors (8-15)
        8 => Color::DarkGray,
        9 => Color::LightRed,
        10 => Color::LightGreen,
        11 => Color::LightYellow,
        12 => Color::LightBlue,
        13 => Color::LightMagenta,
        14 => Color::LightCyan,
        15 => Color::Gray,
        // 216-color cube (16-231): 6x6x6 RGB
        16..=231 => {
            let idx = idx - 16;
            let r = (idx / 36) % 6;
            let g = (idx / 6) % 6;
            let b = idx % 6;
            // Convert 0-5 to 0-255 (0, 95, 135, 175, 215, 255)
            let to_rgb = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            Color::Rgb(to_rgb(r), to_rgb(g), to_rgb(b))
        }
        // Grayscale (232-255): 24 shades from dark to light
        232..=255 => {
            let gray = 8 + (idx - 232) * 10;
            Color::Rgb(gray, gray, gray)
        }
    }
}

/// Render the main scene panel (terminal area).
pub fn render_main_scene(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::MainScene;

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = if let Some(room) = app.selected_room_info() {
        if app.scrollback_offset > 0 {
            format!(" {} [↑{}] ", room.name, app.scrollback_offset)
        } else {
            format!(" {} ", room.name)
        }
    } else {
        " Terminal ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Check if we have a PTY session for the selected room
    if let Some(session) = app.current_session() {
        let screen = session.screen();
        let (screen_rows, screen_cols) = screen.size();

        // Log if there's a mismatch between screen size and rendered area
        if debug_log::is_enabled() && (screen_cols != inner.width || screen_rows != inner.height) {
            debug_log::log_debug(&format!(
                "SIZE MISMATCH: screen={}x{} inner={}x{}",
                screen_cols, screen_rows, inner.width, inner.height
            ));
        }

        // Render cells directly to frame buffer
        let buf = frame.buffer_mut();
        for y in 0..inner.height as usize {
            for x in 0..inner.width as usize {
                let buf_x = inner.x + x as u16;
                let buf_y = inner.y + y as u16;

                // Get cell from vt100 screen if within bounds
                if (y as u16) < screen_rows && (x as u16) < screen_cols {
                    let cell = screen.cell(y as u16, x as u16);
                    if let Some(cell) = cell {
                        let c = cell.contents().chars().next().unwrap_or(' ');
                        let mut fg = vt100_color_to_ratatui(cell.fgcolor(), true);
                        let mut bg = vt100_color_to_ratatui(cell.bgcolor(), false);
                        // Many terminal apps use inverse video (swapped fg/bg) to indicate the cursor
                        // or selections. Honoring cell.inverse() here ensures we render those correctly.
                        if cell.inverse() {
                            std::mem::swap(&mut fg, &mut bg);
                        }
                        if app.selection_contains(y as u16, x as u16) {
                            bg = Color::DarkGray;
                            fg = Color::White;
                        }

                        buf[(buf_x, buf_y)].set_char(c).set_fg(fg).set_bg(bg);
                    } else {
                        buf[(buf_x, buf_y)]
                            .set_char(' ')
                            .set_fg(Color::Reset)
                            .set_bg(Color::Reset);
                    }
                } else {
                    // Outside screen buffer - clear
                    buf[(buf_x, buf_y)]
                        .set_char(' ')
                        .set_fg(Color::Reset)
                        .set_bg(Color::Reset);
                }
            }
        }

        // Note: Cursor positioning is handled in app.rs after all rendering is complete
    } else if let Some(room) = app.selected_room_info() {
        let branch = room.branch.as_deref().unwrap_or("detached");
        let mut content = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("Room: {}", room.name),
                Style::default().fg(Color::White),
            )),
            Line::from(Span::styled(
                format!("Branch: {}", branch),
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
        ];

        if let Some(PendingRoomStatus::Creating) = app.pending_room_status(room) {
            let content = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Creating room...",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("Room: {}", room.name),
                    Style::default().fg(Color::White),
                )),
                Line::from(Span::styled(
                    format!("Branch: {}", branch),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Initializing workspace",
                    Style::default().fg(Color::Gray),
                )),
                Line::from(Span::styled(
                    "Preparing shell",
                    Style::default().fg(Color::Gray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "You'll be connected automatically when ready.",
                    Style::default().fg(Color::DarkGray),
                )),
            ];

            let paragraph = Paragraph::new(content).alignment(Alignment::Center);
            frame.render_widget(paragraph, inner);
            return;
        }

        if let Some(PendingRoomStatus::Failed(error)) = app.pending_room_status(room) {
            let content = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Room creation failed",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("Room: {}", room.name),
                    Style::default().fg(Color::White),
                )),
                Line::from(Span::styled(
                    format!("Branch: {}", branch),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    error.to_string(),
                    Style::default().fg(Color::Red),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Press Enter to retry or D to remove",
                    Style::default().fg(Color::Yellow),
                )),
            ];

            let paragraph = Paragraph::new(content).alignment(Alignment::Center);
            frame.render_widget(paragraph, inner);
            return;
        }

        if app.room_section(room) == RoomSection::Failed {
            let detail = if room.is_prunable {
                PRUNABLE_WORKTREE_MESSAGE.to_string()
            } else if let Some(error) = room.last_error.as_deref() {
                // Show the actual error message when available
                format!("Worktree error: {}", error)
            } else {
                FAILED_WORKTREE_DEFAULT_MESSAGE.to_string()
            };
            content.push(Line::from(Span::styled(
                detail,
                Style::default().fg(Color::Red),
            )));
        } else {
            // No session yet - show info
            content.push(Line::from(Span::styled(
                "Press Enter to start shell",
                Style::default().fg(Color::Yellow),
            )));
        }

        let paragraph = Paragraph::new(content).alignment(Alignment::Center);
        frame.render_widget(paragraph, inner);
    } else {
        // No room selected
        let content = vec![
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
        ];

        let paragraph = Paragraph::new(content).alignment(Alignment::Center);
        frame.render_widget(paragraph, inner);
    }
}
