// Allow dead code for fields that will be used in later implementation steps
#![allow(dead_code)]

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Terminal;

use crate::config::Config;
use crate::git::Worktree;
use crate::room::{create_room, remove_room, CreateRoomOptions, DirtyStatus};
use crate::state::RoomsState;

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

    /// Current rooms state.
    pub state: RoomsState,

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
}

impl App {
    /// Create a new App instance.
    pub fn new(
        repo_root: PathBuf,
        rooms_dir: PathBuf,
        config: Config,
        state: RoomsState,
        worktrees: Vec<Worktree>,
    ) -> Self {
        Self {
            repo_root,
            rooms_dir,
            config,
            state,
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
        }
    }

    /// Run the application main loop.
    pub fn run(&mut self) -> io::Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Main loop
        let result = self.main_loop(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    fn main_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
        loop {
            // Draw UI
            terminal.draw(|frame| self.render(frame))?;

            // Handle input (with 100ms timeout for responsiveness)
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key);
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
                let msg = Paragraph::new("Press Ctrl+B for sidebar, Ctrl+T for terminal, ? for help")
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

    fn calculate_layout(&self, area: Rect) -> Vec<Rect> {
        match (self.sidebar_visible, self.main_scene_visible) {
            (true, true) => {
                // 30% sidebar, 70% main
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
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

        // Global keys (always work)
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
                } else {
                    self.focus = Focus::Sidebar;
                }
                return;
            }
            _ => {}
        }

        // Focus-specific keys
        match self.focus {
            Focus::Sidebar => self.handle_sidebar_key(key),
            Focus::MainScene => self.handle_main_scene_key(key),
        }
    }

    fn handle_prompt_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.prompt.cancel();
            }
            KeyCode::Enter => {
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
                if self.main_scene_visible {
                    self.focus = Focus::MainScene;
                }
            }
            KeyCode::Char('a') => {
                self.prompt = PromptState::start_room_creation();
            }
            KeyCode::Char('A') => {
                self.create_room_silent();
            }
            KeyCode::Char('d') => {
                self.start_room_deletion();
            }
            _ => {}
        }
    }

    fn handle_main_scene_key(&mut self, _key: KeyEvent) {
        // TODO: Forward to PTY when implemented
        // For now, just show a message
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
        self.state.rooms.len()
    }

    /// Get the currently selected room, if any.
    pub fn selected_room(&self) -> Option<&crate::state::Room> {
        self.state.rooms.get(self.selected_index)
    }

    /// Create a new room silently (with generated name).
    fn create_room_silent(&mut self) {
        let options = CreateRoomOptions::default();

        match create_room(&self.rooms_dir, &mut self.state, options) {
            Ok(room) => {
                // Select the new room
                self.selected_index = self.state.rooms.len().saturating_sub(1);

                // Save state
                if let Err(e) = self.state.save_to_rooms_dir(&self.rooms_dir) {
                    self.status_message = Some(format!("Room created but failed to save: {}", e));
                } else {
                    self.status_message = Some(format!("Created room: {}", room.name));
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to create room: {}", e));
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
                // Select the new room
                self.selected_index = self.state.rooms.len().saturating_sub(1);

                // Save state
                if let Err(e) = self.state.save_to_rooms_dir(&self.rooms_dir) {
                    self.status_message = Some(format!("Room created but failed to save: {}", e));
                } else {
                    self.status_message = Some(format!("Created room: {}", room.name));
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to create room: {}", e));
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
            KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::Char('h') | KeyCode::Char('l') => {
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

    /// Delete the room with the given name.
    fn delete_room(&mut self, room_name: &str) {
        // Use force=true since we already warned about dirty status
        match remove_room(&mut self.state, room_name, true) {
            Ok(name) => {
                // Adjust selection if needed
                if self.selected_index >= self.state.rooms.len() && self.selected_index > 0 {
                    self.selected_index -= 1;
                }

                // Save state
                if let Err(e) = self.state.save_to_rooms_dir(&self.rooms_dir) {
                    self.status_message = Some(format!("Room deleted but failed to save: {}", e));
                } else {
                    self.status_message = Some(format!("Deleted room: {}", name));
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to delete room: {}", e));
            }
        }
    }
}
