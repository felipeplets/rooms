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
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Terminal;

use uuid::Uuid;

use crate::config::Config;
use crate::git::Worktree;
use crate::room::{
    create_room, discover_rooms, remove_room, rename_room, run_post_create_commands,
    CreateRoomOptions, DirtyStatus, PostCreateHandle, RoomInfo,
};
use crate::state::{EventLog, RoomStatus, RoomsState, TransientStateStore};
use crate::terminal::PtySession;

use super::confirm::{render_confirm, ConfirmState};
use super::help::render_help;
use super::main_scene::render_main_scene;
use super::prompt::{render_prompt, PromptState};
use super::sidebar::render_sidebar;

/// Which panel currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Focus {
    #[default]
    Sidebar,
    MainScene,
}

/// Application state for the TUI.
pub struct App {
    /// Path to the repository root.
    pub repo_root: PathBuf,

    /// Path to the rooms directory.
    pub rooms_dir: PathBuf,

    /// Application configuration.
    pub config: Config,

    /// Current rooms state (legacy, will be removed).
    pub state: RoomsState,

    /// Discovered rooms from git worktrees.
    pub rooms: Vec<RoomInfo>,

    /// Transient state store for in-memory room states.
    pub transient: TransientStateStore,

    /// Discovered git worktrees.
    pub worktrees: Vec<Worktree>,

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

    /// Last known terminal size for resize detection.
    pub last_size: (u16, u16),

    /// Running post-create operations.
    post_create_handles: Vec<PostCreateHandle>,

    /// Event logger.
    event_log: EventLog,

    /// Whether to skip post-create commands this session.
    skip_post_create: bool,
}

impl App {
    /// Create a new App instance.
    pub fn new(
        repo_root: PathBuf,
        rooms_dir: PathBuf,
        config: Config,
        state: RoomsState,
        worktrees: Vec<Worktree>,
        skip_post_create: bool,
    ) -> Self {
        let event_log = EventLog::new(&rooms_dir);
        let transient = TransientStateStore::new();

        // Discover rooms from git worktrees
        let rooms = match discover_rooms(&repo_root, &rooms_dir, &transient) {
            Ok(rooms) => rooms,
            Err(e) => {
                // Log the error for debugging - the app will start with empty rooms
                event_log.log_error(None, &format!("Failed to discover rooms at startup: {}", e));
                Vec::new()
            }
        };

        Self {
            repo_root,
            rooms_dir,
            config,
            state,
            rooms,
            transient,
            worktrees,
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
            last_size: (0, 0),
            post_create_handles: Vec::new(),
            event_log,
            skip_post_create,
        }
    }

    /// Refresh the rooms list from git worktrees.
    ///
    /// This re-discovers rooms by calling `git worktree list` and merging
    /// with transient state. The current selection is preserved if the
    /// selected room still exists.
    pub fn refresh_rooms(&mut self) {
        let selected_name = self.rooms.get(self.selected_index).map(|r| r.name.clone());

        match discover_rooms(&self.repo_root, &self.rooms_dir, &self.transient) {
            Ok(rooms) => {
                self.rooms = rooms;

                // Restore selection if the room still exists
                if let Some(name) = selected_name {
                    if let Some(idx) = self.rooms.iter().position(|r| r.name == name) {
                        self.selected_index = idx;
                    }
                }

                // Ensure selected_index is valid for the current rooms list
                if self.rooms.is_empty() {
                    self.selected_index = 0;
                } else if self.selected_index >= self.rooms.len() {
                    self.selected_index = self.rooms.len() - 1;
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to refresh rooms: {}", e));
            }
        }
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

            // Poll for post-create command completion
            self.poll_post_create();

            // Update terminal size and resize PTY sessions if needed
            // This handles both terminal resize and layout changes (e.g., sidebar toggle)
            let size = terminal.size()?;
            self.last_size = (size.width, size.height);
            let (cols, rows) = self.calculate_pty_size();
            for session in self.sessions.values_mut() {
                // resize() already checks if dimensions changed and skips if same
                session.resize(cols, rows);
            }

            // Draw UI
            terminal.draw(|frame| self.render(frame))?;

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

        // Get the main scene area (same logic as render())
        let main_area = match (self.sidebar_visible, self.main_scene_visible) {
            (true, true) => chunks.get(1).copied().unwrap_or(area),
            (true, false) => area, // No main scene visible
            (false, true) => chunks.first().copied().unwrap_or(area),
            (false, false) => area,
        };

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
                KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.main_scene_visible = !self.main_scene_visible;
                    if !self.main_scene_visible {
                        self.focus = Focus::Sidebar;
                    }
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
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.main_scene_visible = !self.main_scene_visible;
                if !self.main_scene_visible && self.focus == Focus::MainScene {
                    self.focus = Focus::Sidebar;
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
                if self.main_scene_visible && self.selected_room_info().is_some() {
                    // Start PTY session if not already running
                    let (cols, rows) = self.calculate_pty_size();
                    self.get_or_create_session(cols, rows);
                    self.focus = Focus::MainScene;
                }
            }
            KeyCode::Char('a') => {
                self.prompt = PromptState::start_room_creation();
            }
            KeyCode::Char('A') => {
                self.create_room_silent();
            }
            KeyCode::Delete => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // Ctrl+Delete: delete without confirmation
                    self.delete_room_immediate();
                } else {
                    // Delete: show confirmation dialog
                    self.start_room_deletion();
                }
            }
            KeyCode::Char('r') => {
                self.start_room_rename();
            }
            KeyCode::Char('R') => {
                self.refresh_rooms();
                self.status_message = Some("Rooms refreshed".to_string());
            }
            _ => {}
        }
    }

    fn handle_main_scene_key(&mut self, key: KeyEvent) {
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

        // Send input to PTY
        if let Some(session) = self.current_session_mut() {
            if let Err(e) = session.write(&bytes) {
                self.status_message = Some(format!("Write error: {}", e));
            }
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                // TODO: Implement scrollback viewing with vt100
                // For now, scrolling is handled by the PTY itself
            }
            MouseEventKind::ScrollDown => {
                // TODO: Implement scrollback viewing with vt100
                // For now, scrolling is handled by the PTY itself
            }
            _ => {}
        }
    }

    fn handle_paste(&mut self, text: String) {
        // Only process paste in terminal mode
        if self.focus != Focus::MainScene {
            return;
        }

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
        }
    }

    fn select_previous(&mut self) {
        let total = self.total_items();
        if total > 0 {
            self.selected_index = self.selected_index.checked_sub(1).unwrap_or(total - 1);
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

    /// Get the currently selected room from state (legacy), if any.
    ///
    /// Note: `selected_index` is based on `self.rooms` (RoomInfo list from git worktrees).
    /// This method maps the selection to the legacy `state.rooms` list by looking up
    /// the room by name.
    pub fn selected_room(&self) -> Option<&crate::state::Room> {
        let info = self.selected_room_info()?;
        self.state.rooms.iter().find(|room| room.name == info.name)
    }

    /// Create a new room silently (with generated name).
    fn create_room_silent(&mut self) {
        let options = CreateRoomOptions::default();

        match create_room(&self.rooms_dir, &mut self.state, options) {
            Ok(room) => {
                let room_name = room.name.clone();
                let room_id = room.id;
                let room_path = room.path.clone();

                // Refresh rooms from git worktrees
                self.refresh_rooms();

                // Select the new room
                if let Some(idx) = self.rooms.iter().position(|r| r.name == room_name) {
                    self.selected_index = idx;
                } else {
                    // Room not found after refresh - this shouldn't happen but handle gracefully
                    self.event_log.log_error(
                        Some(&room_name),
                        "Room created but not found in worktree list after refresh",
                    );
                }

                // Log the event
                self.event_log.log_room_created(&room_name);

                // Start post-create commands if configured
                self.start_post_create(&room_name, room_id, room_path);

                // Save state (legacy)
                if let Err(e) = self.state.save_to_rooms_dir(&self.rooms_dir) {
                    self.status_message = Some(format!("Room created but failed to save: {}", e));
                } else {
                    self.status_message = Some(format!("Created room: {}", room_name));
                }
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

        match create_room(&self.rooms_dir, &mut self.state, options) {
            Ok(room) => {
                let room_name = room.name.clone();
                let room_id = room.id;
                let room_path = room.path.clone();

                // Refresh rooms from git worktrees
                self.refresh_rooms();

                // Select the new room
                if let Some(idx) = self.rooms.iter().position(|r| r.name == room_name) {
                    self.selected_index = idx;
                } else {
                    // Room not found after refresh - this shouldn't happen but handle gracefully
                    self.event_log.log_error(
                        Some(&room_name),
                        "Room created but not found in worktree list after refresh",
                    );
                }

                // Log the event
                self.event_log.log_room_created(&room_name);

                // Start post-create commands if configured
                self.start_post_create(&room_name, room_id, room_path);

                // Save state (legacy)
                if let Err(e) = self.state.save_to_rooms_dir(&self.rooms_dir) {
                    self.status_message = Some(format!("Room created but failed to save: {}", e));
                } else {
                    self.status_message = Some(format!("Created room: {}", room_name));
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to create room: {}", e));
                self.event_log.log_error(None, &e.to_string());
            }
        }
    }

    /// Start post-create commands for a newly created room.
    fn start_post_create(&mut self, room_name: &str, room_id: Uuid, room_path: PathBuf) {
        if self.skip_post_create || self.config.post_create_commands.is_empty() {
            return;
        }

        // Update room status
        if let Some(room) = self.state.rooms.iter_mut().find(|r| r.id == room_id) {
            room.status = RoomStatus::PostCreateRunning;
        }

        // Log start
        self.event_log
            .log_post_create_started(room_name, self.config.post_create_commands.len());

        // Start background execution
        let handle = run_post_create_commands(
            room_id,
            room_path,
            self.repo_root.clone(),
            self.config.post_create_commands.clone(),
        );

        self.post_create_handles.push(handle);

        // Save state with updated status
        let _ = self.state.save_to_rooms_dir(&self.rooms_dir);
    }

    /// Poll for post-create command completion.
    fn poll_post_create(&mut self) {
        let mut completed = Vec::new();

        for (i, handle) in self.post_create_handles.iter().enumerate() {
            if let Some(result) = handle.try_recv() {
                completed.push((i, result));
            }
        }

        // Process completed in reverse order to preserve indices
        for (i, result) in completed.into_iter().rev() {
            self.post_create_handles.remove(i);

            // Find the room and get its name for logging
            let room_name = self
                .state
                .rooms
                .iter()
                .find(|r| r.id == result.room_id)
                .map(|r| r.name.clone());

            // Update room status
            if let Some(room) = self.state.rooms.iter_mut().find(|r| r.id == result.room_id) {
                if result.success {
                    room.status = RoomStatus::Ready;
                    room.last_error = None;
                    if let Some(name) = &room_name {
                        self.event_log.log_post_create_completed(name);
                    }
                } else {
                    room.status = RoomStatus::Error;
                    room.last_error = result.error.clone();
                    if let Some(name) = &room_name {
                        self.event_log.log_post_create_failed(
                            name,
                            result.error.as_deref().unwrap_or("unknown error"),
                        );
                    }
                    self.status_message = Some(format!(
                        "Post-create failed: {}",
                        result.error.unwrap_or_else(|| "unknown error".to_string())
                    ));
                }
            }

            // Save updated state
            let _ = self.state.save_to_rooms_dir(&self.rooms_dir);
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
        let room = match self.selected_room() {
            Some(r) => r,
            None => {
                self.status_message = Some("No room selected".to_string());
                return;
            }
        };

        let room_name = room.name.clone();
        let room_path = room.path.to_string_lossy().to_string();
        let branch = room.branch.clone();

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
        let room = match self.selected_room() {
            Some(r) => r,
            None => {
                self.status_message = Some("No room selected".to_string());
                return;
            }
        };

        let room_name = room.name.clone();
        self.delete_room(&room_name);
    }

    /// Delete the room with the given name.
    fn delete_room(&mut self, room_name: &str) {
        // Use force=true since we already warned about dirty status
        match remove_room(&mut self.state, room_name, true) {
            Ok(name) => {
                // Remove PTY session if exists (keyed by room name)
                self.sessions.remove(&name);

                // Log the event
                self.event_log.log_room_deleted(&name);

                // Refresh rooms from git worktrees
                self.refresh_rooms();

                // Save state (legacy)
                if let Err(e) = self.state.save_to_rooms_dir(&self.rooms_dir) {
                    self.status_message = Some(format!("Room deleted but failed to save: {}", e));
                } else {
                    self.status_message = Some(format!("Deleted room: {}", name));
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to delete room: {}", e));
                self.event_log.log_error(Some(room_name), &e.to_string());
            }
        }
    }

    /// Start the room rename flow.
    fn start_room_rename(&mut self) {
        let room = match self.selected_room() {
            Some(r) => r,
            None => {
                self.status_message = Some("No room selected".to_string());
                return;
            }
        };

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

        match rename_room(
            &self.repo_root,
            &self.rooms_dir,
            &mut self.state,
            old_name,
            new_name,
        ) {
            Ok(_) => {
                // Remove PTY session since the working directory changed (keyed by old name)
                self.sessions.remove(old_name);

                // Log the event
                self.event_log.log_room_renamed(old_name, new_name);

                // Refresh rooms from git worktrees
                self.refresh_rooms();

                // Save state (legacy)
                if let Err(e) = self.state.save_to_rooms_dir(&self.rooms_dir) {
                    self.status_message = Some(format!("Room renamed but failed to save: {}", e));
                } else {
                    self.status_message = Some(format!("Renamed: {} -> {}", old_name, new_name));
                }
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
}
