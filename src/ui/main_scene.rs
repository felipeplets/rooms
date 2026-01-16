use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::app::{App, Focus};
use crate::terminal::debug_log;

/// Convert vt100 color to ratatui Color.
fn vt100_color_to_ratatui(color: vt100::Color, is_foreground: bool) -> Color {
    match color {
        vt100::Color::Default => {
            if is_foreground {
                Color::White
            } else {
                Color::Black
            }
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

    let title = if let Some(room) = app.selected_room() {
        format!(" {} ", room.name)
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
                        let fg = vt100_color_to_ratatui(cell.fgcolor(), true);
                        let bg = vt100_color_to_ratatui(cell.bgcolor(), false);

                        buf[(buf_x, buf_y)].set_char(c).set_fg(fg).set_bg(bg);
                    } else {
                        buf[(buf_x, buf_y)]
                            .set_char(' ')
                            .set_fg(Color::White)
                            .set_bg(Color::Black);
                    }
                } else {
                    // Outside screen buffer - clear
                    buf[(buf_x, buf_y)]
                        .set_char(' ')
                        .set_fg(Color::White)
                        .set_bg(Color::Black);
                }
            }
        }

        // Show cursor if focused and visible
        let (cursor_row, cursor_col) = screen.cursor_position();
        if is_focused && !screen.hide_cursor() {
            let cursor_x = inner.x + cursor_col;
            let cursor_y = inner.y + cursor_row;
            if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    } else if let Some(room) = app.selected_room() {
        // No session yet - show info
        let content = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("Room: {}", room.name),
                Style::default().fg(Color::White),
            )),
            Line::from(Span::styled(
                format!("Branch: {}", room.branch),
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press Enter to start shell",
                Style::default().fg(Color::Yellow),
            )),
        ];

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
