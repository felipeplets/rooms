use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

/// State for a text input prompt.
#[derive(Debug, Clone)]
pub struct TextInput {
    /// Current input value.
    pub value: String,

    /// Cursor position in the input.
    pub cursor: usize,

    /// Placeholder text shown when empty.
    pub placeholder: String,
}

impl TextInput {
    pub fn new(placeholder: impl Into<String>) -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            placeholder: placeholder.into(),
        }
    }

    /// Insert a character at the cursor position.
    pub fn insert(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += 1;
    }

    /// Delete the character before the cursor (backspace).
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.value.remove(self.cursor);
        }
    }

    /// Delete the character at the cursor (delete).
    pub fn delete(&mut self) {
        if self.cursor < self.value.len() {
            self.value.remove(self.cursor);
        }
    }

    /// Move cursor left.
    pub fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Move cursor right.
    pub fn move_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor += 1;
        }
    }

    /// Move cursor to start.
    pub fn move_start(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end.
    pub fn move_end(&mut self) {
        self.cursor = self.value.len();
    }

    /// Clear the input.
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    /// Get the current value, or None if empty.
    pub fn get_value(&self) -> Option<String> {
        if self.value.is_empty() {
            None
        } else {
            Some(self.value.clone())
        }
    }
}

/// The current prompt being shown.
#[derive(Debug, Clone, Default)]
pub enum PromptState {
    /// No prompt active.
    #[default]
    None,

    /// Prompting for room name.
    RoomName(TextInput),

    /// Prompting for branch name (after room name).
    BranchName {
        room_name: Option<String>,
        input: TextInput,
    },

    /// Prompting for new room name (rename).
    RenameRoom {
        /// Original name (for lookup during save).
        current_name: String,
        /// Text input pre-filled with current name.
        input: TextInput,
    },
}

impl PromptState {
    /// Start prompting for a new room.
    pub fn start_room_creation() -> Self {
        Self::RoomName(TextInput::new("Leave empty for generated name"))
    }

    /// Start prompting for a room rename.
    pub fn start_room_rename(current_name: String) -> Self {
        let mut input = TextInput::new("");
        input.value = current_name.clone();
        input.cursor = input.value.len(); // Cursor at end
        Self::RenameRoom {
            current_name,
            input,
        }
    }

    /// Check if a prompt is active.
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Get the current text input, if any.
    pub fn current_input(&mut self) -> Option<&mut TextInput> {
        match self {
            Self::None => None,
            Self::RoomName(input) => Some(input),
            Self::BranchName { input, .. } => Some(input),
            Self::RenameRoom { input, .. } => Some(input),
        }
    }

    /// Advance to the next prompt step, returning the final result if done.
    /// Returns Some((room_name, branch_name)) when complete.
    /// Note: RenameRoom is handled separately and should not use this method.
    pub fn advance(&mut self) -> Option<(Option<String>, Option<String>)> {
        match std::mem::take(self) {
            Self::None => None,
            Self::RoomName(input) => {
                let room_name = input.get_value();
                *self = Self::BranchName {
                    room_name,
                    input: TextInput::new("Leave empty to use room name"),
                };
                None
            }
            Self::BranchName { room_name, input } => {
                let branch_name = input.get_value();
                *self = Self::None;
                Some((room_name, branch_name))
            }
            Self::RenameRoom { .. } => {
                // RenameRoom is handled directly in handle_prompt_key, not via advance()
                *self = Self::None;
                None
            }
        }
    }

    /// Cancel the current prompt.
    pub fn cancel(&mut self) {
        *self = Self::None;
    }
}

/// Render the current prompt overlay.
pub fn render_prompt(frame: &mut Frame, area: Rect, prompt: &PromptState) {
    let (title, hint, input) = match prompt {
        PromptState::None => return,
        PromptState::RoomName(input) => ("Create Room - Name", "Enter room name:", input),
        PromptState::BranchName { input, .. } => ("Create Room - Branch", "Enter branch name:", input),
        PromptState::RenameRoom { input, .. } => ("Rename Room", "Enter new name:", input),
    };

    // Center the prompt
    let popup_area = centered_rect(50, 30, area);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Layout: hint, input, help
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(inner);

    // Hint text
    let hint_text = Paragraph::new(hint).style(Style::default().fg(Color::White));
    frame.render_widget(hint_text, chunks[0]);

    // Input field
    let display_value = if input.value.is_empty() {
        Span::styled(&input.placeholder, Style::default().fg(Color::DarkGray))
    } else {
        Span::raw(&input.value)
    };

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let input_paragraph = Paragraph::new(Line::from(display_value))
        .block(input_block);
    frame.render_widget(input_paragraph, chunks[1]);

    // Set cursor position
    if !input.value.is_empty() || input.placeholder.is_empty() {
        let cursor_x = chunks[1].x + 1 + input.cursor as u16;
        let cursor_y = chunks[1].y + 1;
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    // Help text
    let help = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" confirm  "),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" cancel"),
        ]),
    ])
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_input_basic() {
        let mut input = TextInput::new("placeholder");
        assert!(input.value.is_empty());
        assert_eq!(input.cursor, 0);

        input.insert('h');
        input.insert('i');
        assert_eq!(input.value, "hi");
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn test_text_input_backspace() {
        let mut input = TextInput::new("");
        input.insert('a');
        input.insert('b');
        input.insert('c');
        input.backspace();
        assert_eq!(input.value, "ab");
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn test_text_input_cursor_movement() {
        let mut input = TextInput::new("");
        input.insert('a');
        input.insert('b');
        input.insert('c');

        input.move_left();
        assert_eq!(input.cursor, 2);

        input.move_start();
        assert_eq!(input.cursor, 0);

        input.move_end();
        assert_eq!(input.cursor, 3);
    }

    #[test]
    fn test_prompt_state_flow() {
        let mut prompt = PromptState::start_room_creation();
        assert!(prompt.is_active());

        // Enter room name
        if let Some(input) = prompt.current_input() {
            input.insert('t');
            input.insert('e');
            input.insert('s');
            input.insert('t');
        }

        // Advance to branch
        let result = prompt.advance();
        assert!(result.is_none());
        assert!(matches!(prompt, PromptState::BranchName { .. }));

        // Leave branch empty and advance
        let result = prompt.advance();
        assert!(result.is_some());
        let (room_name, branch_name) = result.unwrap();
        assert_eq!(room_name, Some("test".to_string()));
        assert_eq!(branch_name, None);
    }
}
