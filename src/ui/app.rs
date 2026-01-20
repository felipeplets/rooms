// Allow dead code for fields that will be used in later implementation steps
#![allow(dead_code)]

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::config::Config;
use crate::git::prune_worktrees_from;
use crate::room::{
    CreateRoomOptions, DirtyStatus, RoomInfo, RoomStatus, create_room, discover_rooms, remove_room,
    rename_room,
};
use crate::state::{EventLog, TransientStateStore};
use crate::terminal::PtySession;

use super::clipboard::{copy_to_clipboard, paste_from_clipboard};
use super::confirm::{ConfirmState, render_confirm};
use super::context_menu::{ContextMenuItem, ContextMenuState};
use super::help::render_help;
use super::main_scene::render_main_scene;
use super::prompt::{PromptState, render_prompt};
use super::selection::{Selection, SelectionBounds};
use super::sidebar::render_sidebar;

/// Maximum scrollback lines for the PTY terminal.
const SCROLLBACK_LINES: usize = 1000;

/// Which panel currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Focus {
    #[default]
    Sidebar,
    MainScene,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomSection {
    Active,
    Inactive,
    Failed,
}

#[derive(Debug, Clone, Copy)]
enum SelectionMove {
    Left,
    Right,
    Up,
    Down,
}

/// Application state for the TUI.
pub struct App {
    /// Path to the repository root.
    pub repo_root: PathBuf,

    /// Path to the rooms directory.
    pub rooms_dir: PathBuf,

    /// Application configuration.
    pub config: Config,

    /// Discovered rooms from git worktrees.
    pub rooms: Vec<RoomInfo>,

    /// Transient state store for in-memory room states.
    pub transient: TransientStateStore,

    /// Primary worktree path.
    pub primary_worktree: PathBuf,

    /// Currently selected room index.
    pub selected_index: usize,

    /// Which panel has focus.
    pub focus: Focus,

    /// Whether the sidebar is visible.
    pub sidebar_visible: bool,

    /// Whether the main scene is visible.
    pub main_scene_visible: bool,

    /// Whether the help overlay is shown.
    pub show_help: bool,

    /// Whether the app should quit.
    pub should_quit: bool,

    /// Status message to display.
    pub status_message: Option<String>,

    /// Current prompt state for interactive input.
    pub prompt: PromptState,

    /// Current confirmation dialog state.
    pub confirm: ConfirmState,

    /// PTY sessions per room (keyed by room name).
    pub sessions: HashMap<String, PtySession>,

    /// Scrollback offset for the current session (0 = at bottom, >0 = scrolled up).
    pub scrollback_offset: usize,

    /// Previous scrollback offset to detect changes.
    prev_scrollback_offset: usize,

    /// Last known terminal size for resize detection.
    pub last_size: (u16, u16),

    /// Event logger.
    event_log: EventLog,

    /// Whether to skip lifecycle hooks this session.
    skip_hooks: bool,

    /// Active text selection in the PTY, if any.
    selection: Option<Selection>,

    /// Whether a selection drag is in progress.
    selection_dragging: bool,

    /// Start position for a pending selection drag.
    selection_anchor: Option<(u16, u16)>,

    /// Context menu state for PTY selection.
    context_menu: Option<ContextMenuState>,
}

impl App {
    /// Create a new App instance.
    pub fn new(
        repo_root: PathBuf,
        rooms_dir: PathBuf,
        config: Config,
        primary_worktree: PathBuf,
        skip_hooks: bool,
    ) -> Self {
        let event_log = EventLog::new(&rooms_dir);
        let transient = TransientStateStore::new();

        // Discover rooms from git worktrees
        let rooms =
            match discover_rooms(&repo_root, &rooms_dir, Some(&primary_worktree), &transient) {
                Ok(rooms) => rooms,
                Err(e) => {
                    // Log the error for debugging - the app will start with empty rooms
                    event_log
                        .log_error(None, &format!("Failed to discover rooms at startup: {}", e));
                    Vec::new()
                }
            };

        let mut app = Self {
            repo_root,
            rooms_dir,
            config,
            rooms,
            transient,
            primary_worktree,
            selected_index: 0,
            focus: Focus::default(),
            sidebar_visible: true,
            main_scene_visible: true,
            show_help: false,
            should_quit: false,
            status_message: None,
            prompt: PromptState::default(),
            confirm: ConfirmState::default(),
            sessions: HashMap::new(),
            scrollback_offset: 0,
            prev_scrollback_offset: 0,
            last_size: (0, 0),
            event_log,
            skip_hooks,
            selection: None,
            selection_dragging: false,
            selection_anchor: None,
            context_menu: None,
        };

        app.sort_rooms_for_sidebar();
        app
    }

    /// Refresh the rooms list from git worktrees.
    ///
    /// This re-discovers rooms by calling `git worktree list` and merging
    /// with transient state. The current selection is preserved if the
    /// selected room still exists.
    ///
    /// Returns `true` if the refresh succeeded, `false` if it failed.
    pub fn refresh_rooms(&mut self) -> bool {
        let selected_name = self.rooms.get(self.selected_index).map(|r| r.name.clone());

        match discover_rooms(
            &self.repo_root,
            &self.rooms_dir,
            Some(&self.primary_worktree),
            &self.transient,
        ) {
            Ok(rooms) => {
                self.rooms = rooms;
                self.sort_rooms_for_sidebar();

                // Restore selection if the room still exists
                if let Some(name) = selected_name
                    && let Some(idx) = self.rooms.iter().position(|r| r.name == name)
                {
                    self.selected_index = idx;
                }

                // Ensure selected_index is valid for the current rooms list
                if self.rooms.is_empty() {
                    self.selected_index = 0;
                } else if self.selected_index >= self.rooms.len() {
                    self.selected_index = self.rooms.len() - 1;
                }
                true
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to refresh rooms: {}", e));
                false
            }
        }
    }

    fn sort_rooms_for_sidebar(&mut self) {
        let selected_name = self.rooms.get(self.selected_index).map(|r| r.name.clone());
        let active_rooms: std::collections::HashSet<String> =
            self.sessions.keys().cloned().collect();
        let primary_canonical = self.primary_worktree.canonicalize().ok();
        let primary_normalized = normalize_path_for_compare(&self.primary_worktree);

        self.rooms.sort_by(|a, b| {
            let a_primary =
                is_primary_worktree(&a.path, primary_canonical.as_deref(), &primary_normalized);
            let b_primary =
                is_primary_worktree(&b.path, primary_canonical.as_deref(), &primary_normalized);
            let a_key = (
                room_section_rank_with_active(a, &active_rooms),
                if a_primary { 0 } else { 1 },
                a.name.to_lowercase(),
            );
            let b_key = (
                room_section_rank_with_active(b, &active_rooms),
                if b_primary { 0 } else { 1 },
                b.name.to_lowercase(),
            );
            a_key.cmp(&b_key)
        });

        if let Some(name) = selected_name
            && let Some(idx) = self.rooms.iter().position(|room| room.name == name)
        {
            self.selected_index = idx;
        }

        // Ensure selected_index is valid for the current rooms list
        if self.rooms.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= self.rooms.len() {
            self.selected_index = self.rooms.len() - 1;
        }
    }

    pub fn room_section(&self, room: &RoomInfo) -> RoomSection {
        if self.room_is_failed(room) {
            RoomSection::Failed
        } else if self.sessions.contains_key(&room.name) {
            RoomSection::Active
        } else {
            RoomSection::Inactive
        }
    }

    fn room_is_failed(&self, room: &RoomInfo) -> bool {
        room.is_prunable
            || matches!(room.status, RoomStatus::Error | RoomStatus::Orphaned)
            || room.last_error.is_some()
    }

    /// Run the application main loop.
    pub fn run(&mut self) -> io::Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste
        )?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Force a full clear to sync ratatui's internal state with the actual terminal
        terminal.clear()?;

        // Ensure cursor is shown initially
        terminal.show_cursor()?;

        // Main loop
        let result = self.main_loop(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            DisableBracketedPaste
        )?;
        terminal.show_cursor()?;

        result
    }

    fn main_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> io::Result<()> {
        loop {
            // Process PTY output for all sessions
            for session in self.sessions.values_mut() {
                session.process_output();
            }

            // Update terminal size and resize PTY sessions if needed
            // This handles both terminal resize and layout changes (e.g., sidebar toggle)
            let size = terminal.size()?;
            self.last_size = (size.width, size.height);
            let (cols, rows) = self.calculate_pty_size();
            for session in self.sessions.values_mut() {
                // resize() already checks if dimensions changed and skips if same
                session.resize(cols, rows);
            }

            // Apply scrollback offset to the current session (only if changed)
            if let Some(room_info) = self.selected_room_info() {
                let offset = self.scrollback_offset;
                // Only update if offset changed
                if offset != self.prev_scrollback_offset {
                    // Clone is necessary to avoid holding a borrow while mutably accessing sessions
                    let room_name = room_info.name.clone();
                    if let Some(session) = self.sessions.get_mut(&room_name) {
                        session.screen_mut().set_scrollback(offset);
                    }
                    self.prev_scrollback_offset = offset;
                }
            }

            // Draw UI
            terminal.draw(|frame| self.render(frame))?;

            // Set cursor visibility after draw based on PTY state
            // Ratatui positions the cursor during draw, so we control visibility after
            if self.should_show_cursor() {
                terminal.show_cursor()?;
            } else {
                terminal.hide_cursor()?;
            }

            // Handle input (with 50ms timeout for PTY responsiveness)
            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => self.handle_key(key),
                    Event::Mouse(mouse) => self.handle_mouse(mouse),
                    Event::Paste(text) => self.handle_paste(text),
                    _ => {}
                }
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn render(&self, frame: &mut ratatui::Frame) {
        let area = frame.area();

        // If help is shown, render it as overlay
        if self.show_help {
            render_help(frame, area);
            return;
        }

        // If prompt is active, render it as overlay
        if self.prompt.is_active() {
            render_prompt(frame, area, &self.prompt);
            return;
        }

        // If confirmation dialog is active, render it as overlay
        if self.confirm.is_active() {
            render_confirm(frame, area, &self.confirm);
            return;
        }

        // Calculate layout based on panel visibility
        let chunks = self.calculate_layout(area);

        // Render panels
        match (self.sidebar_visible, self.main_scene_visible) {
            (true, true) => {
                render_sidebar(frame, chunks[0], self);
                render_main_scene(frame, chunks[1], self);
            }
            (true, false) => {
                render_sidebar(frame, chunks[0], self);
            }
            (false, true) => {
                render_main_scene(frame, chunks[0], self);
            }
            (false, false) => {
                // Show minimal status when both panels hidden
                let msg =
                    Paragraph::new("Press Ctrl+B for sidebar, Ctrl+T for terminal, ? for help")
                        .style(Style::default().fg(Color::DarkGray))
                        .block(Block::default().borders(Borders::ALL).title("rooms"));
                frame.render_widget(msg, area);
            }
        }

        if let Some(menu) = &self.context_menu {
            self.render_context_menu(frame, menu);
        }

        // Render status message if present
        if let Some(ref msg) = self.status_message {
            let status_area = Rect {
                x: area.x,
                y: area.height.saturating_sub(1),
                width: area.width,
                height: 1,
            };
            let status = Paragraph::new(msg.as_str()).style(Style::default().fg(Color::Yellow));
            frame.render_widget(status, status_area);
        }

        // PTY cursor handling - must be done AFTER all rendering
        // Set cursor position when PTY wants cursor visible
        if self.should_show_cursor()
            && let Some(session) = self.current_session()
        {
            let screen = session.screen();

            // Calculate which area is the main scene
            let main_area = Self::get_main_scene_area(
                area,
                &chunks,
                self.sidebar_visible,
                self.main_scene_visible,
            );

            // Calculate inner area (subtract borders)
            let inner = Rect {
                x: main_area.x.saturating_add(1),
                y: main_area.y.saturating_add(1),
                width: main_area.width.saturating_sub(2),
                height: main_area.height.saturating_sub(2),
            };

            // Only position cursor if viewport has non-zero dimensions
            if inner.width > 0 && inner.height > 0 {
                // Get PTY cursor position
                let (cursor_row, cursor_col) = screen.cursor_position();

                // Clamp to visible viewport
                let cursor_col = cursor_col.min(inner.width.saturating_sub(1));
                let cursor_row = cursor_row.min(inner.height.saturating_sub(1));

                // Set cursor position
                frame.set_cursor_position((inner.x + cursor_col, inner.y + cursor_row));
            }
        }
    }

    /// Helper method to get the main scene area from the layout chunks.
    /// This logic is shared between cursor positioning and PTY size calculation.
    fn get_main_scene_area(
        area: Rect,
        chunks: &[Rect],
        sidebar_visible: bool,
        main_scene_visible: bool,
    ) -> Rect {
        match (sidebar_visible, main_scene_visible) {
            (true, true) => chunks.get(1).copied().unwrap_or(area),
            (false, true) => chunks.first().copied().unwrap_or(area),
            _ => area,
        }
    }

    /// Calculate the PTY size based on current terminal size and sidebar visibility.
    /// This must match the actual rendered area inside the terminal block (with borders).
    /// We replicate exactly what render does: calculate_layout() then block.inner().
    fn calculate_pty_size(&self) -> (u16, u16) {
        // Replicate the exact area calculation from render
        let area = Rect {
            x: 0,
            y: 0,
            width: self.last_size.0,
            height: self.last_size.1,
        };
        let chunks = self.calculate_layout(area);

        // Get the main scene area using the shared helper method
        let main_area =
            Self::get_main_scene_area(area, &chunks, self.sidebar_visible, self.main_scene_visible);

        // Calculate inner area after block borders (Borders::ALL subtracts 2 from each dimension)
        let inner_width = main_area.width.saturating_sub(2);
        let inner_height = main_area.height.saturating_sub(2);

        (inner_width.max(10), inner_height.max(5))
    }

    fn calculate_layout(&self, area: Rect) -> Vec<Rect> {
        match (self.sidebar_visible, self.main_scene_visible) {
            (true, true) => {
                // Fixed 40-column sidebar, main takes remaining space
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Length(40), Constraint::Fill(1)])
                    .split(area)
                    .to_vec()
            }
            (true, false) | (false, true) => {
                // Full width for single panel
                vec![area]
            }
            (false, false) => {
                vec![area]
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.handle_context_menu_key(key) {
            return;
        }

        // Handle confirmation dialog first if active
        if self.confirm.is_active() {
            self.handle_confirm_key(key);
            return;
        }

        // Handle prompt input if active
        if self.prompt.is_active() {
            self.handle_prompt_key(key);
            return;
        }

        // When focused on MainScene (PTY), forward most keys to the terminal
        // Ctrl+B focuses sidebar (and shows it if hidden), Ctrl+T toggles terminal
        if self.focus == Focus::MainScene {
            match key.code {
                KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    // Focus sidebar and ensure it's visible
                    self.sidebar_visible = true;
                    self.focus = Focus::Sidebar;
                }
                _ => self.handle_main_scene_key(key),
            }
            return;
        }

        // Global keys (when NOT focused on MainScene)
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                return;
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
                return;
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.sidebar_visible = !self.sidebar_visible;
                // If hiding the focused panel, switch focus
                if !self.sidebar_visible && self.focus == Focus::Sidebar {
                    self.focus = Focus::MainScene;
                }
                return;
            }
            KeyCode::Esc => {
                if self.show_help {
                    self.show_help = false;
                }
                return;
            }
            _ => {}
        }

        // Focus-specific keys (only Sidebar reaches here now)
        self.handle_sidebar_key(key);
    }

    fn handle_prompt_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.prompt.cancel();
            }
            KeyCode::Enter => {
                // Handle RenameRoom separately (single-step prompt)
                if let PromptState::RenameRoom {
                    current_name,
                    input,
                } = &self.prompt
                {
                    let old_name = current_name.clone();
                    let new_name = input.value.clone();
                    self.prompt = PromptState::None;
                    self.apply_room_rename(&old_name, &new_name);
                    return;
                }

                if let Some((room_name, branch_name)) = self.prompt.advance() {
                    // Prompt complete, create the room
                    self.create_room_interactive(room_name, branch_name);
                }
            }
            KeyCode::Backspace => {
                if let Some(input) = self.prompt.current_input() {
                    input.backspace();
                }
            }
            KeyCode::Delete => {
                if let Some(input) = self.prompt.current_input() {
                    input.delete();
                }
            }
            KeyCode::Left => {
                if let Some(input) = self.prompt.current_input() {
                    input.move_left();
                }
            }
            KeyCode::Right => {
                if let Some(input) = self.prompt.current_input() {
                    input.move_right();
                }
            }
            KeyCode::Home => {
                if let Some(input) = self.prompt.current_input() {
                    input.move_start();
                }
            }
            KeyCode::End => {
                if let Some(input) = self.prompt.current_input() {
                    input.move_end();
                }
            }
            KeyCode::Char(c) => {
                if let Some(input) = self.prompt.current_input() {
                    input.insert(c);
                }
            }
            _ => {}
        }
    }

    fn handle_sidebar_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.select_next();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.select_previous();
            }
            KeyCode::Enter => {
                if !self.main_scene_visible {
                    return;
                }

                let Some(room) = self.selected_room_info() else {
                    return;
                };

                if self.room_section(room) == RoomSection::Failed {
                    if room.is_prunable {
                        match prune_worktrees_from(&self.repo_root) {
                            Ok(_) => {
                                self.refresh_rooms();
                                self.status_message = Some("Ran git worktree prune".to_string());
                            }
                            Err(e) => {
                                self.status_message =
                                    Some(format!("Failed to prune worktrees: {e}"));
                            }
                        }
                    } else {
                        self.status_message = Some("Cannot open failed worktree".to_string());
                    }
                    return;
                }

                // Start PTY session if not already running
                self.enter_selected_room(false);
            }
            KeyCode::Char('a') => {
                self.prompt = PromptState::start_room_creation();
            }
            KeyCode::Char('A') => {
                self.create_room_silent();
            }
            KeyCode::Char('d') | KeyCode::Delete | KeyCode::Backspace => {
                self.start_room_deletion();
            }
            KeyCode::Char('D') => {
                self.delete_room_immediate();
            }
            KeyCode::Char('r') => {
                self.start_room_rename();
            }
            KeyCode::Char('R') => {
                if self.refresh_rooms() {
                    self.status_message = Some("Rooms refreshed".to_string());
                }
            }
            _ => {}
        }
    }

    fn handle_main_scene_key(&mut self, key: KeyEvent) {
        if self.handle_selection_key(key) {
            return;
        }

        // Handle scrollback navigation keys (don't forward to PTY)
        match key.code {
            KeyCode::PageUp => {
                if let Some(session) = self.current_session() {
                    let screen = session.screen();
                    let (rows, _cols) = screen.size();
                    // Scroll up by one page (screen height)
                    self.scrollback_offset =
                        (self.scrollback_offset + rows as usize).min(SCROLLBACK_LINES);
                }
                return;
            }
            KeyCode::PageDown => {
                if let Some(session) = self.current_session() {
                    let screen = session.screen();
                    let (rows, _cols) = screen.size();
                    // Scroll down by one page (screen height)
                    self.scrollback_offset = self.scrollback_offset.saturating_sub(rows as usize);
                }
                return;
            }
            _ => {}
        }

        // Convert key event to bytes and send to PTY
        let bytes = match key.code {
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // Ctrl+letter sends ASCII 1-26
                    let ctrl_char = (c.to_ascii_lowercase() as u8).wrapping_sub(b'a' - 1);
                    vec![ctrl_char]
                } else {
                    let mut buf = [0u8; 4];
                    c.encode_utf8(&mut buf).as_bytes().to_vec()
                }
            }
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::ALT) {
                    // Alt+Enter: send ESC + CR for literal newline in shell
                    vec![0x1b, b'\r']
                } else {
                    vec![b'\r']
                }
            }
            KeyCode::Backspace => vec![0x7f],
            KeyCode::Tab => vec![b'\t'],
            KeyCode::Up => vec![0x1b, b'[', b'A'],
            KeyCode::Down => vec![0x1b, b'[', b'B'],
            KeyCode::Right => vec![0x1b, b'[', b'C'],
            KeyCode::Left => vec![0x1b, b'[', b'D'],
            KeyCode::Home => vec![0x1b, b'[', b'H'],
            KeyCode::End => vec![0x1b, b'[', b'F'],
            KeyCode::PageUp => vec![0x1b, b'[', b'5', b'~'],
            KeyCode::PageDown => vec![0x1b, b'[', b'6', b'~'],
            KeyCode::Delete => vec![0x1b, b'[', b'3', b'~'],
            KeyCode::Insert => vec![0x1b, b'[', b'2', b'~'],
            KeyCode::F(n) => match n {
                1 => vec![0x1b, b'O', b'P'],
                2 => vec![0x1b, b'O', b'Q'],
                3 => vec![0x1b, b'O', b'R'],
                4 => vec![0x1b, b'O', b'S'],
                5 => vec![0x1b, b'[', b'1', b'5', b'~'],
                6 => vec![0x1b, b'[', b'1', b'7', b'~'],
                7 => vec![0x1b, b'[', b'1', b'8', b'~'],
                8 => vec![0x1b, b'[', b'1', b'9', b'~'],
                9 => vec![0x1b, b'[', b'2', b'0', b'~'],
                10 => vec![0x1b, b'[', b'2', b'1', b'~'],
                11 => vec![0x1b, b'[', b'2', b'3', b'~'],
                12 => vec![0x1b, b'[', b'2', b'4', b'~'],
                _ => return,
            },
            _ => return,
        };

        self.write_to_pty(&bytes, true);
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.handle_context_menu_mouse(mouse) {
            return;
        }

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                // Only scroll in terminal mode
                if self.focus == Focus::MainScene
                    && let Some(_session) = self.current_session()
                {
                    // Scroll up by 3 lines at a time
                    self.scrollback_offset = (self.scrollback_offset + 3).min(SCROLLBACK_LINES);
                }
            }
            MouseEventKind::ScrollDown => {
                // Only scroll in terminal mode
                if self.focus == Focus::MainScene
                    && let Some(_session) = self.current_session()
                {
                    // Scroll down by 3 lines, minimum 0 (at bottom)
                    self.scrollback_offset = self.scrollback_offset.saturating_sub(3);
                }
            }
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                let position = self.mouse_to_screen_position(mouse);
                if let Some((row, col)) = position {
                    if !self.selection_contains(row, col) {
                        self.clear_selection();
                    }
                    self.start_selection(mouse);
                } else {
                    self.clear_selection();
                }
            }
            MouseEventKind::Drag(crossterm::event::MouseButton::Left) => {
                self.update_selection(mouse);
            }
            MouseEventKind::Up(crossterm::event::MouseButton::Left) => {
                self.end_selection();
            }
            MouseEventKind::Down(crossterm::event::MouseButton::Right) => {
                self.open_context_menu(mouse);
            }
            _ => {}
        }
    }

    fn handle_paste(&mut self, text: String) {
        // Only process paste in terminal mode
        if self.focus != Focus::MainScene {
            return;
        }

        self.clear_selection();

        if let Some(session) = self.current_session_mut() {
            // Send bracketed paste start sequence
            let start = b"\x1b[200~";
            // Send bracketed paste end sequence
            let end = b"\x1b[201~";

            // Write: start marker + content + end marker
            let _ = session.write(start);
            let _ = session.write(text.as_bytes());
            if let Err(e) = session.write(end) {
                self.status_message = Some(format!("Paste error: {}", e));
            }
        }
    }

    fn select_next(&mut self) {
        let total = self.total_items();
        if total > 0 {
            self.selected_index = (self.selected_index + 1) % total;
            self.scrollback_offset = 0; // Reset scrollback when changing rooms
            self.prev_scrollback_offset = 0;
        }
    }

    fn select_previous(&mut self) {
        let total = self.total_items();
        if total > 0 {
            self.selected_index = self.selected_index.checked_sub(1).unwrap_or(total - 1);
            self.scrollback_offset = 0; // Reset scrollback when changing rooms
            self.prev_scrollback_offset = 0;
        }
    }

    /// Get total number of selectable items (rooms).
    pub fn total_items(&self) -> usize {
        self.rooms.len()
    }

    /// Get the currently selected room (RoomInfo), if any.
    pub fn selected_room_info(&self) -> Option<&RoomInfo> {
        self.rooms.get(self.selected_index)
    }

    /// Create a new room silently (with generated name).
    fn create_room_silent(&mut self) {
        let options = CreateRoomOptions::default();

        match create_room(&self.repo_root, &self.rooms_dir, options) {
            Ok(room) => {
                let room_name = room.name.clone();

                // Refresh rooms from git worktrees
                self.refresh_rooms();

                // Select the new room
                let mut selected = false;
                if let Some(idx) = self.rooms.iter().position(|r| r.name == room_name) {
                    self.selected_index = idx;
                    selected = true;
                } else {
                    // Room not found after refresh - this shouldn't happen but handle gracefully
                    self.event_log.log_error(
                        Some(&room_name),
                        "Room created but not found in worktree list after refresh",
                    );
                }

                // Log the event
                self.event_log.log_room_created(&room_name);

                // Auto-enter the new room and run hooks.
                if selected {
                    self.enter_selected_room(true);
                }
                self.status_message = Some(format!("Created room: {}", room_name));
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to create room: {}", e));
                self.event_log.log_error(None, &e.to_string());
            }
        }
    }

    /// Create a new room with user-provided name and branch.
    fn create_room_interactive(&mut self, room_name: Option<String>, branch_name: Option<String>) {
        let options = CreateRoomOptions {
            name: room_name,
            branch: branch_name,
            ..Default::default()
        };

        match create_room(&self.repo_root, &self.rooms_dir, options) {
            Ok(room) => {
                let room_name = room.name.clone();

                // Refresh rooms from git worktrees
                self.refresh_rooms();

                // Select the new room
                let mut selected = false;
                if let Some(idx) = self.rooms.iter().position(|r| r.name == room_name) {
                    self.selected_index = idx;
                    selected = true;
                } else {
                    // Room not found after refresh - this shouldn't happen but handle gracefully
                    self.event_log.log_error(
                        Some(&room_name),
                        "Room created but not found in worktree list after refresh",
                    );
                }

                // Log the event
                self.event_log.log_room_created(&room_name);

                // Auto-enter the new room and run hooks.
                if selected {
                    self.enter_selected_room(true);
                }
                self.status_message = Some(format!("Created room: {}", room_name));
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to create room: {}", e));
                self.event_log.log_error(None, &e.to_string());
            }
        }
    }

    fn enter_selected_room(&mut self, run_post_create: bool) {
        if !self.main_scene_visible {
            self.main_scene_visible = true;
        }

        let (cols, rows) = self.calculate_pty_size();
        if self.get_or_create_session(cols, rows).is_none() {
            return;
        }

        self.focus = Focus::MainScene;

        let post_create = self.config.hooks.post_create.clone();
        let post_enter = self.config.hooks.post_enter.clone();

        if run_post_create {
            self.run_hook_commands(&post_create);
        }
        self.run_hook_commands(&post_enter);
    }

    fn run_hook_commands(&mut self, commands: &[String]) {
        if self.skip_hooks || commands.is_empty() {
            return;
        }

        for command in commands {
            if command.ends_with('\n') {
                self.write_to_pty(command.as_bytes(), false);
            } else {
                let mut line = command.clone();
                line.push('\n');
                self.write_to_pty(line.as_bytes(), false);
            }
        }
    }

    fn handle_confirm_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.confirm.cancel();
            }
            KeyCode::Enter => {
                if let Some(room_name) = self.confirm.confirm() {
                    self.delete_room(&room_name);
                }
            }
            KeyCode::Left
            | KeyCode::Right
            | KeyCode::Tab
            | KeyCode::Char('h')
            | KeyCode::Char('l') => {
                self.confirm.toggle_selection();
            }
            KeyCode::Char('y') => {
                // Quick confirm with 'y'
                if let ConfirmState::DeleteRoom { room_name, .. } = &self.confirm {
                    let name = room_name.clone();
                    self.confirm.cancel();
                    self.delete_room(&name);
                }
            }
            KeyCode::Char('n') => {
                // Quick cancel with 'n'
                self.confirm.cancel();
            }
            _ => {}
        }
    }

    /// Start the room deletion flow.
    fn start_room_deletion(&mut self) {
        let room = match self.selected_room_info() {
            Some(r) => r,
            None => {
                self.status_message = Some("No room selected".to_string());
                return;
            }
        };

        if room.is_primary {
            self.status_message = Some("Cannot delete the primary worktree".to_string());
            return;
        }

        let room_name = room.name.clone();
        let room_path = room.path.to_string_lossy().to_string();
        let branch = room
            .branch
            .clone()
            .unwrap_or_else(|| "detached".to_string());

        // Check dirty status
        let dirty_status = match DirtyStatus::check(&room.path) {
            Ok(status) => Some(status),
            Err(e) => {
                self.status_message = Some(format!("Warning: couldn't check status: {}", e));
                None
            }
        };

        self.confirm = ConfirmState::start_delete(room_name, room_path, branch, dirty_status);
    }

    /// Delete the currently selected room immediately without confirmation.
    fn delete_room_immediate(&mut self) {
        let room = match self.selected_room_info() {
            Some(r) => r,
            None => {
                self.status_message = Some("No room selected".to_string());
                return;
            }
        };

        if room.is_primary {
            self.status_message = Some("Cannot delete the primary worktree".to_string());
            return;
        }

        let room_name = room.name.clone();
        self.delete_room(&room_name);
    }

    /// Delete the room with the given name.
    fn delete_room(&mut self, room_name: &str) {
        // Use force=true since we already warned about dirty status
        match remove_room(&self.repo_root, &self.rooms_dir, room_name, true) {
            Ok(name) => {
                // Remove PTY session if exists (keyed by room name)
                self.sessions.remove(&name);
                self.transient.remove(&name);

                // Log the event
                self.event_log.log_room_deleted(&name);

                // Refresh rooms from git worktrees
                self.refresh_rooms();
                self.status_message = Some(format!("Deleted room: {}", name));
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to delete room: {}", e));
                self.event_log.log_error(Some(room_name), &e.to_string());
            }
        }
    }

    /// Start the room rename flow.
    fn start_room_rename(&mut self) {
        let room = match self.selected_room_info() {
            Some(r) => r,
            None => {
                self.status_message = Some("No room selected".to_string());
                return;
            }
        };

        if room.is_primary {
            self.status_message = Some("Cannot rename the primary worktree".to_string());
            return;
        }

        let current_name = room.name.clone();
        self.prompt = PromptState::start_room_rename(current_name);
    }

    /// Apply a room rename.
    fn apply_room_rename(&mut self, old_name: &str, new_name: &str) {
        // Skip if new name is empty
        if new_name.is_empty() {
            self.status_message = Some("Rename cancelled: name cannot be empty".to_string());
            return;
        }

        match rename_room(&self.repo_root, &self.rooms_dir, old_name, new_name) {
            Ok(_) => {
                // Remove PTY session since the working directory changed (keyed by old name)
                self.sessions.remove(old_name);
                self.transient.remove(old_name);

                // Log the event
                self.event_log.log_room_renamed(old_name, new_name);

                // Refresh rooms from git worktrees
                self.refresh_rooms();
                self.status_message = Some(format!("Renamed: {} -> {}", old_name, new_name));
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to rename room: {}", e));
                self.event_log.log_error(Some(old_name), &e.to_string());
            }
        }
    }

    /// Get or create a PTY session for the selected room.
    pub fn get_or_create_session(&mut self, cols: u16, rows: u16) -> Option<&mut PtySession> {
        use std::collections::hash_map::Entry;

        let room = self.selected_room_info()?;
        let room_name = room.name.clone();
        let room_path = room.path.clone();

        if let Entry::Vacant(entry) = self.sessions.entry(room_name.clone()) {
            match PtySession::new(cols, rows, &room_path) {
                Ok(session) => {
                    entry.insert(session);
                    self.sort_rooms_for_sidebar();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to start shell: {}", e));
                    return None;
                }
            }
        }

        self.sessions.get_mut(&room_name)
    }

    /// Get the PTY session for the selected room, if it exists.
    pub fn current_session(&self) -> Option<&PtySession> {
        let room = self.selected_room_info()?;
        self.sessions.get(&room.name)
    }

    /// Get the PTY session for the selected room mutably, if it exists.
    pub fn current_session_mut(&mut self) -> Option<&mut PtySession> {
        let room_name = self.selected_room_info()?.name.clone();
        self.sessions.get_mut(&room_name)
    }

    /// Determine if the terminal cursor should be visible.
    fn should_show_cursor(&self) -> bool {
        // Show cursor when a prompt is active
        if self.prompt.is_active() {
            return true;
        }

        // Don't show cursor when not focused on main scene
        if self.focus != Focus::MainScene || !self.main_scene_visible {
            return false;
        }

        // Don't show cursor when viewing scrollback
        if self.scrollback_offset != 0 {
            return false;
        }

        let Some(session) = self.current_session() else {
            return false;
        };

        let screen = session.screen();

        // Show cursor when PTY wants it visible.
        !screen.hide_cursor()
    }

    fn main_scene_inner_rect(&self) -> Rect {
        let area = Rect {
            x: 0,
            y: 0,
            width: self.last_size.0,
            height: self.last_size.1,
        };
        let chunks = self.calculate_layout(area);
        let main_area =
            Self::get_main_scene_area(area, &chunks, self.sidebar_visible, self.main_scene_visible);
        Rect {
            x: main_area.x.saturating_add(1),
            y: main_area.y.saturating_add(1),
            width: main_area.width.saturating_sub(2),
            height: main_area.height.saturating_sub(2),
        }
    }

    fn selection_bounds(&self) -> Option<SelectionBounds> {
        let selection = self.selection.as_ref()?;
        Some(selection.bounds())
    }

    pub fn selection_contains(&self, row: u16, col: u16) -> bool {
        let Some(bounds) = self.selection_bounds() else {
            return false;
        };
        bounds.contains(row, col)
    }

    fn clear_selection(&mut self) {
        self.selection = None;
        self.selection_dragging = false;
        self.selection_anchor = None;
    }

    fn start_selection(&mut self, mouse: MouseEvent) {
        if self.focus != Focus::MainScene || self.scrollback_offset != 0 {
            return;
        }
        if self.current_session().is_none() {
            return;
        }
        let Some(position) = self.mouse_to_screen_position(mouse) else {
            return;
        };
        if self.selection_contains(position.0, position.1) {
            self.selection_anchor = Some(position);
            return;
        }
        self.clear_selection();
        self.selection_anchor = Some(position);
    }

    fn update_selection(&mut self, mouse: MouseEvent) {
        let Some(anchor) = self.selection_anchor else {
            return;
        };
        let Some(position) = self.mouse_to_screen_position(mouse) else {
            return;
        };
        if position == anchor {
            self.selection = None;
            self.selection_dragging = false;
            return;
        }
        self.selection = Some(Selection {
            start: anchor,
            end: position,
        });
        self.selection_dragging = true;
    }

    fn end_selection(&mut self) {
        self.selection_dragging = false;
        self.selection_anchor = None;
    }

    fn open_context_menu(&mut self, mouse: MouseEvent) {
        if self.focus != Focus::MainScene {
            return;
        }

        let mut menu_items = Vec::new();
        if self.selection.is_some() {
            menu_items.push(ContextMenuItem::Copy);
        }
        menu_items.push(ContextMenuItem::Paste);

        self.context_menu = Some(ContextMenuState {
            items: menu_items,
            selected: 0,
            position: (mouse.column, mouse.row),
        });
    }

    fn handle_context_menu_key(&mut self, key: KeyEvent) -> bool {
        let Some(menu) = &mut self.context_menu else {
            return false;
        };

        match key.code {
            KeyCode::Esc => {
                self.context_menu = None;
            }
            KeyCode::Up => {
                menu.selected = menu.selected.saturating_sub(1);
            }
            KeyCode::Down => {
                if menu.selected + 1 < menu.items.len() {
                    menu.selected += 1;
                }
            }
            KeyCode::Enter => {
                let action = menu.items.get(menu.selected).copied();
                self.context_menu = None;
                if let Some(action) = action {
                    self.apply_context_menu_action(action);
                }
            }
            _ => {}
        }

        true
    }

    fn handle_context_menu_mouse(&mut self, mouse: MouseEvent) -> bool {
        let Some(menu) = &self.context_menu else {
            return false;
        };
        if !matches!(
            mouse.kind,
            MouseEventKind::Down(crossterm::event::MouseButton::Left)
        ) {
            return false;
        }

        let menu_rect = self.context_menu_rect(menu);
        if !menu_rect.contains((mouse.column, mouse.row).into()) {
            self.context_menu = None;
            return true;
        }

        let index = mouse
            .row
            .saturating_sub(menu_rect.y + 1)
            .min(menu.items.len().saturating_sub(1) as u16) as usize;
        let action = menu.items.get(index).copied();
        self.context_menu = None;
        if let Some(action) = action {
            self.apply_context_menu_action(action);
        }
        true
    }

    fn apply_context_menu_action(&mut self, action: ContextMenuItem) {
        match action {
            ContextMenuItem::Copy => {
                if let Some(text) = self.selection_text() {
                    match copy_to_clipboard(&text) {
                        Ok(()) => {
                            self.status_message = Some("Selection copied".to_string());
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Copy failed: {}", e));
                        }
                    }
                } else {
                    self.status_message = Some("No selection to copy".to_string());
                }
            }
            ContextMenuItem::Paste => match paste_from_clipboard() {
                Ok(text) => self.handle_paste(text),
                Err(e) => self.status_message = Some(format!("Paste failed: {}", e)),
            },
        }
    }

    fn selection_text(&self) -> Option<String> {
        let selection = self.selection.as_ref()?;
        let session = self.current_session()?;
        let screen = session.screen();
        let (rows, cols) = screen.size();
        if rows == 0 || cols == 0 {
            return None;
        }

        let bounds = selection.bounds();
        let max_row = rows.saturating_sub(1);
        let max_col = cols.saturating_sub(1);
        let bounds = SelectionBounds {
            start_row: bounds.start_row.min(max_row),
            start_col: bounds.start_col.min(max_col),
            end_row: bounds.end_row.min(max_row),
            end_col: bounds.end_col.min(max_col),
        };
        let mut lines = Vec::new();

        for row in bounds.start_row..=bounds.end_row {
            let mut line = String::new();
            let col_start = if row == bounds.start_row {
                bounds.start_col
            } else {
                0
            };
            let col_end = if row == bounds.end_row {
                bounds.end_col
            } else {
                cols.saturating_sub(1)
            };

            for col in col_start..=col_end {
                if let Some(cell) = screen.cell(row, col) {
                    line.push(cell.contents().chars().next().unwrap_or(' '));
                } else {
                    line.push(' ');
                }
            }

            lines.push(line.trim_end().to_string());
        }

        Some(lines.join("\n"))
    }

    fn mouse_to_screen_position(&self, mouse: MouseEvent) -> Option<(u16, u16)> {
        let inner = self.main_scene_inner_rect();
        if !inner.contains((mouse.column, mouse.row).into()) {
            return None;
        }
        let col = mouse.column.saturating_sub(inner.x);
        let row = mouse.row.saturating_sub(inner.y);
        Some((row, col))
    }

    fn handle_selection_key(&mut self, key: KeyEvent) -> bool {
        if self.focus != Focus::MainScene {
            return false;
        }

        if key
            .modifiers
            .intersects(KeyModifiers::SUPER | KeyModifiers::META)
            && let KeyCode::Char('c') = key.code
        {
            if let Some(text) = self.selection_text() {
                match copy_to_clipboard(&text) {
                    Ok(()) => {
                        self.status_message = Some("Selection copied".to_string());
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Copy failed: {}", e));
                    }
                }
            } else {
                self.status_message = Some("No selection to copy".to_string());
            }
            return true;
        }

        if !key.modifiers.contains(KeyModifiers::SHIFT) {
            return false;
        }

        let direction = match key.code {
            KeyCode::Left => SelectionMove::Left,
            KeyCode::Right => SelectionMove::Right,
            KeyCode::Up => SelectionMove::Up,
            KeyCode::Down => SelectionMove::Down,
            _ => return false,
        };

        let Some(session) = self.current_session() else {
            return true;
        };
        let screen = session.screen();
        let (rows, cols) = screen.size();
        if rows == 0 || cols == 0 {
            return true;
        }

        let (anchor, current) = if let Some(selection) = self.selection {
            (selection.start, selection.end)
        } else {
            let cursor = screen.cursor_position();
            (cursor, cursor)
        };

        let next = move_selection_position(current, direction, rows, cols);
        self.selection_anchor = Some(anchor);
        self.selection = Some(Selection {
            start: anchor,
            end: next,
        });
        self.selection_dragging = true;
        let arrow_bytes = match direction {
            SelectionMove::Left => vec![0x1b, b'[', b'D'],
            SelectionMove::Right => vec![0x1b, b'[', b'C'],
            SelectionMove::Up => vec![0x1b, b'[', b'A'],
            SelectionMove::Down => vec![0x1b, b'[', b'B'],
        };
        self.write_to_pty(&arrow_bytes, false);
        true
    }

    fn write_to_pty(&mut self, bytes: &[u8], clear_selection: bool) {
        // Reset scrollback when user types (they're interacting with live terminal)
        self.scrollback_offset = 0;
        self.prev_scrollback_offset = 0;
        if clear_selection {
            self.clear_selection();
        }

        if let Some(session) = self.current_session_mut()
            && let Err(e) = session.write(bytes)
        {
            self.status_message = Some(format!("Write error: {}", e));
        }
    }

    fn context_menu_rect(&self, menu: &ContextMenuState) -> Rect {
        let label_width = menu
            .items
            .iter()
            .map(|item| item.label().len() as u16)
            .max()
            .unwrap_or(0);
        let width = (label_width + 4).max(12);
        let height = menu.items.len() as u16 + 2;
        let area = Rect {
            x: 0,
            y: 0,
            width: self.last_size.0,
            height: self.last_size.1,
        };
        let mut x = menu.position.0;
        let mut y = menu.position.1;
        if x + width > area.width {
            x = area.width.saturating_sub(width);
        }
        if y + height > area.height {
            y = area.height.saturating_sub(height);
        }
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    fn render_context_menu(&self, frame: &mut ratatui::Frame, menu: &ContextMenuState) {
        use ratatui::widgets::{Block, Borders, Clear, List, ListItem};

        let rect = self.context_menu_rect(menu);
        frame.render_widget(Clear, rect);

        let items: Vec<ListItem> = menu
            .items
            .iter()
            .map(|item| ListItem::new(item.label()))
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" Selection "))
            .highlight_style(Style::default().bg(Color::DarkGray));

        let mut state = ratatui::widgets::ListState::default();
        state.select(Some(menu.selected));
        frame.render_stateful_widget(list, rect, &mut state);
    }
}

fn move_selection_position(
    current: (u16, u16),
    direction: SelectionMove,
    rows: u16,
    cols: u16,
) -> (u16, u16) {
    if rows == 0 || cols == 0 {
        return current;
    }

    let max_row = rows.saturating_sub(1);
    let max_col = cols.saturating_sub(1);
    let (mut row, mut col) = current;
    row = row.min(max_row);
    col = col.min(max_col);

    match direction {
        SelectionMove::Left => {
            if col > 0 {
                (row, col - 1)
            } else if row > 0 {
                (row - 1, max_col)
            } else {
                (row, col)
            }
        }
        SelectionMove::Right => {
            if col < max_col {
                (row, col + 1)
            } else if row < max_row {
                (row + 1, 0)
            } else {
                (row, col)
            }
        }
        SelectionMove::Up => {
            if row > 0 {
                (row - 1, col)
            } else {
                (row, col)
            }
        }
        SelectionMove::Down => {
            if row < max_row {
                (row + 1, col)
            } else {
                (row, col)
            }
        }
    }
}

fn room_section_rank_with_active(
    room: &RoomInfo,
    active_rooms: &std::collections::HashSet<String>,
) -> u8 {
    if room.is_prunable
        || matches!(room.status, RoomStatus::Error | RoomStatus::Orphaned)
        || room.last_error.is_some()
    {
        2
    } else if active_rooms.contains(&room.name) {
        0
    } else {
        1
    }
}

fn is_primary_worktree(
    path: &std::path::Path,
    primary_canonical: Option<&std::path::Path>,
    primary_normalized: &str,
) -> bool {
    if let Some(primary) = primary_canonical
        && let Ok(path_canonical) = path.canonicalize()
    {
        return path_canonical == primary;
    }

    normalize_path_for_compare(path) == primary_normalized
}

fn normalize_path_for_compare(path: &std::path::Path) -> String {
    let path_str = path.to_string_lossy();
    let mut normalized = String::with_capacity(path_str.len());
    let mut last_was_sep = false;
    for ch in path_str.chars() {
        let is_sep = ch == '/' || ch == '\\';
        if is_sep {
            if !last_was_sep {
                normalized.push('/');
            }
        } else {
            normalized.push(ch);
        }
        last_was_sep = is_sep;
    }
    if normalized.ends_with('/') {
        normalized.pop();
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_normalize_path_simple() {
        let path = PathBuf::from("/home/user/repo");
        assert_eq!(normalize_path_for_compare(&path), "/home/user/repo");
    }

    #[test]
    fn test_normalize_path_trailing_separator() {
        let path = PathBuf::from("/home/user/repo/");
        assert_eq!(normalize_path_for_compare(&path), "/home/user/repo");
    }

    #[test]
    fn test_normalize_path_multiple_separators() {
        let path = PathBuf::from("/home//user///repo");
        assert_eq!(normalize_path_for_compare(&path), "/home/user/repo");
    }

    #[test]
    fn test_normalize_path_backslashes() {
        // Simulate Windows-style paths
        let path_str = "C:\\Users\\user\\repo";
        let path = std::path::Path::new(path_str);
        let normalized = normalize_path_for_compare(path);
        // Should normalize backslashes to forward slashes
        assert!(!normalized.contains("\\\\"));
    }

    #[test]
    fn test_normalize_path_mixed_separators() {
        let path_str = "/home/user\\repo/subfolder";
        let path = std::path::Path::new(path_str);
        let normalized = normalize_path_for_compare(path);
        // Should handle mixed separators
        assert!(!normalized.contains("\\"));
    }

    #[test]
    fn test_normalize_path_empty() {
        let path = PathBuf::from("");
        assert_eq!(normalize_path_for_compare(&path), "");
    }

    #[test]
    fn test_is_primary_worktree_same_path() {
        let path = PathBuf::from("/home/user/repo");
        let normalized = normalize_path_for_compare(&path);
        assert!(is_primary_worktree(&path, None, &normalized));
    }

    #[test]
    fn test_is_primary_worktree_different_path() {
        let path1 = PathBuf::from("/home/user/repo");
        let path2 = PathBuf::from("/home/user/other");
        let normalized = normalize_path_for_compare(&path2);
        assert!(!is_primary_worktree(&path1, None, &normalized));
    }

    #[test]
    fn test_is_primary_worktree_with_trailing_slash() {
        let path1 = PathBuf::from("/home/user/repo/");
        let path2 = PathBuf::from("/home/user/repo");
        let normalized = normalize_path_for_compare(&path2);
        assert!(is_primary_worktree(&path1, None, &normalized));
    }

    #[test]
    fn test_is_primary_worktree_with_multiple_separators() {
        let path1 = PathBuf::from("/home//user///repo");
        let path2 = PathBuf::from("/home/user/repo");
        let normalized = normalize_path_for_compare(&path2);
        assert!(is_primary_worktree(&path1, None, &normalized));
    }
}
