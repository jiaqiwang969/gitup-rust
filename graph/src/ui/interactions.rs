pub enum Action {
    CursorUp,
    CursorDown,
    SelectCommit,
    ExpandBranch(String),
    CollapseBranch(String),
    JumpToHead,
    JumpToBranch(String),
}

pub struct InteractionHandler {
    selected: Option<String>,
    expanded_branches: Vec<String>,
}

impl InteractionHandler {
    pub fn handle_key(&mut self, key: char) -> Option<Action> {
        match key {
            'k' => Some(Action::CursorUp),
            'j' => Some(Action::CursorDown),
            ' ' => Some(Action::SelectCommit),
            'H' => Some(Action::JumpToHead),
            _ => None,
        }
    }
}
