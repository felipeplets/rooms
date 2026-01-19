#[derive(Debug, Clone, Copy)]
pub enum ContextMenuItem {
    Copy,
    Paste,
}

impl ContextMenuItem {
    pub fn label(self) -> &'static str {
        match self {
            ContextMenuItem::Copy => "Copy",
            ContextMenuItem::Paste => "Paste",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContextMenuState {
    pub items: Vec<ContextMenuItem>,
    pub selected: usize,
    pub position: (u16, u16),
}
