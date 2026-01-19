//! Debug logging for PTY rendering investigation.
//!
//! This module provides logging facilities to diagnose terminal rendering issues.
//! Enable with `--debug-pty` flag. Logs are written to `~/.rooms/debug.log`.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Global flag to enable/disable debug logging.
static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

/// Global log file handle.
static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);

/// Initialize the debug logging system.
pub fn init() -> std::io::Result<()> {
    let log_path = get_log_path();

    // Create parent directory if needed
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)?;

    *LOG_FILE.lock().unwrap() = Some(file);
    DEBUG_ENABLED.store(true, Ordering::SeqCst);

    log_raw(&format!(
        "=== PTY Debug Log Started at {} ===\n",
        timestamp()
    ));

    Ok(())
}

/// Check if debug logging is enabled.
pub fn is_enabled() -> bool {
    DEBUG_ENABLED.load(Ordering::SeqCst)
}

/// Get the log file path.
fn get_log_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".rooms")
        .join("debug.log")
}

/// Get current timestamp as string.
fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs();
            let millis = d.subsec_millis();
            format!("{}.{:03}", secs, millis)
        })
        .unwrap_or_else(|_| "0.000".to_string())
}

/// Write raw message to log.
fn log_raw(msg: &str) {
    if !is_enabled() {
        return;
    }
    if let Ok(mut guard) = LOG_FILE.lock()
        && let Some(ref mut file) = *guard
    {
        let _ = file.write_all(msg.as_bytes());
        let _ = file.flush();
    }
}

/// Log with category prefix.
fn log_with_category(category: &str, msg: &str) {
    if !is_enabled() {
        return;
    }
    log_raw(&format!("[{}] {} {}\n", timestamp(), category, msg));
}

/// Log raw bytes received from PTY.
pub fn log_pty_input(data: &[u8]) {
    if !is_enabled() {
        return;
    }
    // Log as hex dump for non-printable chars, but also show printable chars
    let hex: String = data
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ");
    let printable: String = data
        .iter()
        .map(|&b| {
            if (0x20..0x7f).contains(&b) {
                b as char
            } else if b == 0x1b {
                '^'
            } else if b == 0x0a {
                'n'
            } else if b == 0x0d {
                'r'
            } else {
                '.'
            }
        })
        .collect();
    log_with_category(
        "PTY-IN",
        &format!("len={} hex=[{}] ascii=[{}]", data.len(), hex, printable),
    );
}

/// Log VTE CSI sequence.
#[allow(dead_code)]
pub fn log_vte_csi(action: char, params: &[u16], intermediates: &[u8]) {
    if !is_enabled() {
        return;
    }
    let params_str: String = params
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(";");
    let inter_str: String = intermediates.iter().map(|&b| b as char).collect();
    log_with_category("VTE", &format!("CSI {}{}{}", inter_str, params_str, action));
}

/// Log VTE execute byte (control character).
#[allow(dead_code)]
pub fn log_vte_execute(byte: u8) {
    if !is_enabled() {
        return;
    }
    let name = match byte {
        0x08 => "BS",
        0x09 => "TAB",
        0x0a => "LF",
        0x0b => "VT",
        0x0c => "FF",
        0x0d => "CR",
        _ => "?",
    };
    log_with_category("VTE", &format!("EXEC 0x{:02x} ({})", byte, name));
}

/// Log VTE ESC sequence.
#[allow(dead_code)]
pub fn log_vte_esc(byte: u8, intermediates: &[u8]) {
    if !is_enabled() {
        return;
    }
    let inter_str: String = intermediates.iter().map(|&b| b as char).collect();
    log_with_category("VTE", &format!("ESC {}{}", inter_str, byte as char));
}

/// Log screen clear operation.
#[allow(dead_code)]
pub fn log_screen_clear(operation: &str, mode: u16, cursor: (usize, usize)) {
    if !is_enabled() {
        return;
    }
    log_with_category(
        "SCREEN",
        &format!(
            "{} mode={} cursor=({},{})",
            operation, mode, cursor.0, cursor.1
        ),
    );
}

/// Log line delete/insert operation.
#[allow(dead_code)]
pub fn log_screen_lines(
    operation: &str,
    count: usize,
    cursor_y: usize,
    scroll_region: Option<(usize, usize)>,
) {
    if !is_enabled() {
        return;
    }
    let region = scroll_region
        .map(|(t, b)| format!("({},{})", t, b))
        .unwrap_or_else(|| "none".to_string());
    log_with_category(
        "SCREEN",
        &format!(
            "{} count={} y={} region={}",
            operation, count, cursor_y, region
        ),
    );
}

/// Log scroll operation.
#[allow(dead_code)]
pub fn log_screen_scroll(direction: &str, count: usize, region: Option<(usize, usize)>) {
    if !is_enabled() {
        return;
    }
    let region_str = region
        .map(|(t, b)| format!("({},{})", t, b))
        .unwrap_or_else(|| "full".to_string());
    log_with_category(
        "SCREEN",
        &format!("SCROLL_{} count={} region={}", direction, count, region_str),
    );
}

/// Log cursor movement.
#[allow(dead_code)]
pub fn log_cursor_move(old: (usize, usize), new: (usize, usize), reason: &str) {
    if !is_enabled() {
        return;
    }
    log_with_category(
        "CURSOR",
        &format!(
            "({},{}) -> ({},{}) [{}]",
            old.0, old.1, new.0, new.1, reason
        ),
    );
}

/// Log rendering info.
#[allow(dead_code)]
pub fn log_render_info(total_lines: usize, empty_lines: usize, screen_size: (usize, usize)) {
    if !is_enabled() {
        return;
    }
    log_with_category(
        "RENDER",
        &format!(
            "lines={} empty={} screen={}x{}",
            total_lines, empty_lines, screen_size.0, screen_size.1
        ),
    );
}

/// Log PTY size calculation.
#[allow(dead_code)]
pub fn log_pty_size(terminal_size: (u16, u16), pty_size: (u16, u16), context: &str) {
    if !is_enabled() {
        return;
    }
    log_with_category(
        "SIZE",
        &format!(
            "terminal={}x{} pty={}x{} [{}]",
            terminal_size.0, terminal_size.1, pty_size.0, pty_size.1, context
        ),
    );
}

/// Log PTY resize event.
pub fn log_pty_resize(old: (usize, usize), new: (u16, u16)) {
    if !is_enabled() {
        return;
    }
    log_with_category(
        "SIZE",
        &format!("RESIZE {}x{} -> {}x{}", old.0, old.1, new.0, new.1),
    );
}

/// Log alternate screen mode change.
#[allow(dead_code)]
pub fn log_alternate_screen(entering: bool) {
    if !is_enabled() {
        return;
    }
    log_with_category(
        "SCREEN",
        if entering {
            "ENTER_ALTERNATE_SCREEN"
        } else {
            "LEAVE_ALTERNATE_SCREEN"
        },
    );
}

/// Log a custom debug message.
pub fn log_debug(msg: &str) {
    if !is_enabled() {
        return;
    }
    log_with_category("DEBUG", msg);
}

/// Log a sample of cell contents from a specific row.
#[allow(dead_code)]
pub fn log_cell_row(row: usize, cells: &[char], cols: usize) {
    if !is_enabled() {
        return;
    }
    // Show first 20 chars with their unicode codepoints
    let sample: String = cells
        .iter()
        .take(20.min(cols))
        .map(|&c| if c == ' ' { '·' } else { c })
        .collect();
    let codepoints: String = cells
        .iter()
        .take(20.min(cols))
        .map(|&c| format!("{:04X}", c as u32))
        .collect::<Vec<_>>()
        .join(" ");
    log_with_category(
        "CELLS",
        &format!("row={} sample=[{}] codes=[{}]", row, sample, codepoints),
    );
}

/// Log cell with background color info for a row.
#[allow(dead_code)]
pub fn log_cell_colors(row: usize, bg_colors: &[String]) {
    if !is_enabled() {
        return;
    }
    // Show first 20 background colors
    let sample: String = bg_colors
        .iter()
        .take(20)
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(",");
    log_with_category("COLORS", &format!("row={} bg=[{}]", row, sample));
}
