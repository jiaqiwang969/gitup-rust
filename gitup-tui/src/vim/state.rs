use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use anyhow::Result;
use super::{VimMode, VimAction, Motion, Operator, Position, SelectionRange, SelectionMode};
use super::register::RegisterManager;
use super::marks::MarkManager;
use super::commands::CommandPalette;

/// Core Vim state management
pub struct VimState {
    mode: VimMode,
    operator: Option<Operator>,
    count: Option<usize>,
    register: char,

    // For multi-key sequences
    last_key: Option<char>,
    last_key2: Option<char>,

    // Visual mode state
    visual_anchor: Option<Position>,
    visual_mode_type: Option<SelectionMode>,

    // Command/search buffers
    command_buffer: String,
    search_pattern: String,
    search_backward: bool,

    // History
    command_history: Vec<String>,
    search_history: Vec<String>,
    last_command: Vec<KeyEvent>,

    // Current position
    cursor: Position,
}

impl VimState {
    pub fn new() -> Self {
        Self {
            mode: VimMode::Normal,
            operator: None,
            count: None,
            register: '"',
            last_key: None,
            last_key2: None,
            visual_anchor: None,
            visual_mode_type: None,
            command_buffer: String::new(),
            search_pattern: String::new(),
            search_backward: false,
            command_history: Vec::new(),
            search_history: Vec::new(),
            last_command: Vec::new(),
            cursor: Position::new(0, 0),
        }
    }

    pub fn mode(&self) -> VimMode {
        self.mode
    }

    pub fn mode_line(&self) -> String {
        match self.mode {
            VimMode::Normal => {
                if let Some(count) = self.count {
                    format!("-- NORMAL -- {}", count)
                } else {
                    "-- NORMAL --".to_string()
                }
            }
            VimMode::Insert => "-- INSERT --".to_string(),
            VimMode::Visual => "-- VISUAL --".to_string(),
            VimMode::VisualLine => "-- VISUAL LINE --".to_string(),
            VimMode::VisualBlock => "-- VISUAL BLOCK --".to_string(),
            VimMode::Command => format!(":{}", self.command_buffer),
            VimMode::Search => {
                let prefix = if self.search_backward { "?" } else { "/" };
                format!("{}{}", prefix, self.search_pattern)
            }
            VimMode::Operator => {
                if let Some(ref op) = self.operator {
                    format!("-- {} --", op.to_string())
                } else {
                    "-- OPERATOR --".to_string()
                }
            }
        }
    }

    pub fn handle_normal_key(
        &mut self,
        key: KeyEvent,
        registers: &mut RegisterManager,
        marks: &mut MarkManager,
    ) -> Result<VimAction> {
        // Handle numeric prefix for count
        if let KeyCode::Char(c) = key.code {
            if c.is_ascii_digit() {
                let digit = c.to_digit(10).unwrap() as usize;
                self.count = Some(self.count.unwrap_or(0) * 10 + digit);
                return Ok(VimAction::None);
            }
        }

        let count = self.count.unwrap_or(1);
        let action = match key.code {
            // Basic navigation
            KeyCode::Char('h') => VimAction::Move(Motion::Left(count)),
            KeyCode::Char('j') => VimAction::Move(Motion::Down(count)),
            KeyCode::Char('k') => VimAction::Move(Motion::Up(count)),
            KeyCode::Char('l') => VimAction::Move(Motion::Right(count)),

            // Word navigation
            KeyCode::Char('w') => VimAction::Move(Motion::WordForward(count)),
            KeyCode::Char('b') => VimAction::Move(Motion::WordBackward(count)),
            KeyCode::Char('e') => VimAction::Move(Motion::WordEnd(count)),
            KeyCode::Char('W') => VimAction::Move(Motion::WORDForward(count)),
            KeyCode::Char('B') => VimAction::Move(Motion::WORDBackward(count)),
            KeyCode::Char('E') => VimAction::Move(Motion::WORDEnd(count)),

            // Line navigation
            KeyCode::Char('0') if self.count.is_none() => VimAction::Move(Motion::LineStart),
            KeyCode::Char('^') => VimAction::Move(Motion::LineFirstNonBlank),
            KeyCode::Char('$') => VimAction::Move(Motion::LineEnd),

            // Jump navigation
            KeyCode::Char('g') => {
                if self.last_key == Some('g') {
                    self.last_key = None;
                    VimAction::Move(Motion::FileStart)
                } else {
                    self.last_key = Some('g');
                    VimAction::None
                }
            }
            KeyCode::Char('G') => {
                if self.count.is_some() {
                    VimAction::Move(Motion::Line(count))
                } else {
                    VimAction::Move(Motion::FileEnd)
                }
            }

            // Page navigation
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                VimAction::Move(Motion::PageDown)
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                VimAction::Move(Motion::PageUp)
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                VimAction::Move(Motion::HalfPageDown)
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                VimAction::Move(Motion::HalfPageUp)
            }

            // Visual modes
            KeyCode::Char('v') => {
                self.mode = VimMode::Visual;
                self.visual_anchor = Some(self.cursor);
                self.visual_mode_type = Some(SelectionMode::Character);
                VimAction::ModeChange(VimMode::Visual)
            }
            KeyCode::Char('V') => {
                self.mode = VimMode::VisualLine;
                self.visual_anchor = Some(self.cursor);
                self.visual_mode_type = Some(SelectionMode::Line);
                VimAction::ModeChange(VimMode::VisualLine)
            }
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.mode = VimMode::VisualBlock;
                self.visual_anchor = Some(self.cursor);
                self.visual_mode_type = Some(SelectionMode::Block);
                VimAction::ModeChange(VimMode::VisualBlock)
            }

            // Insert modes
            KeyCode::Char('i') => {
                self.mode = VimMode::Insert;
                VimAction::ModeChange(VimMode::Insert)
            }
            KeyCode::Char('I') => {
                self.mode = VimMode::Insert;
                self.cursor.col = 0; // Move to line start
                VimAction::ModeChange(VimMode::Insert)
            }
            KeyCode::Char('a') => {
                self.mode = VimMode::Insert;
                self.cursor.col += 1; // Move after cursor
                VimAction::ModeChange(VimMode::Insert)
            }
            KeyCode::Char('A') => {
                self.mode = VimMode::Insert;
                // Move to line end
                VimAction::ModeChange(VimMode::Insert)
            }
            KeyCode::Char('o') => {
                self.mode = VimMode::Insert;
                // Open line below
                VimAction::ModeChange(VimMode::Insert)
            }
            KeyCode::Char('O') => {
                self.mode = VimMode::Insert;
                // Open line above
                VimAction::ModeChange(VimMode::Insert)
            }

            // Operators
            KeyCode::Char('d') => {
                self.mode = VimMode::Operator;
                self.operator = Some(Operator::Delete);
                VimAction::None
            }
            KeyCode::Char('y') => {
                self.mode = VimMode::Operator;
                self.operator = Some(Operator::Yank);
                VimAction::None
            }
            KeyCode::Char('c') => {
                self.mode = VimMode::Operator;
                self.operator = Some(Operator::Change);
                VimAction::None
            }

            // Command and search
            KeyCode::Char(':') => {
                self.mode = VimMode::Command;
                self.command_buffer.clear();
                VimAction::ModeChange(VimMode::Command)
            }
            KeyCode::Char('/') => {
                self.mode = VimMode::Search;
                self.search_pattern.clear();
                self.search_backward = false;
                VimAction::ModeChange(VimMode::Search)
            }
            KeyCode::Char('?') => {
                self.mode = VimMode::Search;
                self.search_pattern.clear();
                self.search_backward = true;
                VimAction::ModeChange(VimMode::Search)
            }

            // Marks
            KeyCode::Char('m') => {
                self.mode = VimMode::Operator;
                self.operator = Some(Operator::Mark);
                VimAction::None
            }
            KeyCode::Char('\'') | KeyCode::Char('`') => {
                self.mode = VimMode::Operator;
                self.operator = Some(Operator::JumpToMark);
                VimAction::None
            }

            // Undo/Redo
            KeyCode::Char('u') => VimAction::None, // TODO: Implement undo
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                VimAction::None // TODO: Implement redo
            }

            _ => VimAction::None,
        };

        // Reset count after action
        if !matches!(action, VimAction::None) {
            self.count = None;
            self.last_key = None;
        }

        Ok(action)
    }

    pub fn handle_insert_key(&mut self, key: KeyEvent) -> Result<VimAction> {
        match key.code {
            KeyCode::Esc => {
                self.mode = VimMode::Normal;
                Ok(VimAction::ModeChange(VimMode::Normal))
            }
            KeyCode::Char(c) => Ok(VimAction::Insert(c.to_string())),
            KeyCode::Enter => Ok(VimAction::Insert("\n".to_string())),
            KeyCode::Tab => Ok(VimAction::Insert("\t".to_string())),
            KeyCode::Backspace => Ok(VimAction::Insert("\x08".to_string())), // Backspace character
            _ => Ok(VimAction::None),
        }
    }

    pub fn handle_visual_key(
        &mut self,
        key: KeyEvent,
        registers: &mut RegisterManager,
    ) -> Result<VimAction> {
        match key.code {
            KeyCode::Esc => {
                self.mode = VimMode::Normal;
                self.visual_anchor = None;
                self.visual_mode_type = None;
                Ok(VimAction::ModeChange(VimMode::Normal))
            }

            // Navigation extends selection
            KeyCode::Char('h') => Ok(VimAction::Move(Motion::Left(1))),
            KeyCode::Char('j') => Ok(VimAction::Move(Motion::Down(1))),
            KeyCode::Char('k') => Ok(VimAction::Move(Motion::Up(1))),
            KeyCode::Char('l') => Ok(VimAction::Move(Motion::Right(1))),

            // Operations on selection
            KeyCode::Char('d') => {
                self.mode = VimMode::Normal;
                let selection = self.get_selection();
                self.visual_anchor = None;
                // TODO: Implement delete operation
                Ok(VimAction::None)
            }
            KeyCode::Char('y') => {
                self.mode = VimMode::Normal;
                let selection = self.get_selection();
                self.visual_anchor = None;
                // TODO: Implement yank operation
                Ok(VimAction::None)
            }
            KeyCode::Char('c') => {
                let selection = self.get_selection();
                self.mode = VimMode::Insert;
                self.visual_anchor = None;
                // TODO: Implement change operation
                Ok(VimAction::None)
            }

            _ => Ok(VimAction::None),
        }
    }

    pub fn handle_command_key(
        &mut self,
        key: KeyEvent,
        commands: &mut CommandPalette,
    ) -> Result<VimAction> {
        match key.code {
            KeyCode::Esc => {
                self.mode = VimMode::Normal;
                self.command_buffer.clear();
                Ok(VimAction::ModeChange(VimMode::Normal))
            }
            KeyCode::Enter => {
                let command = self.command_buffer.clone();
                self.command_history.push(command.clone());
                self.command_buffer.clear();
                self.mode = VimMode::Normal;
                Ok(VimAction::ExecuteCommand(command))
            }
            KeyCode::Char(c) => {
                self.command_buffer.push(c);
                Ok(VimAction::None)
            }
            KeyCode::Backspace => {
                self.command_buffer.pop();
                Ok(VimAction::None)
            }
            _ => Ok(VimAction::None),
        }
    }

    pub fn handle_search_key(&mut self, key: KeyEvent) -> Result<VimAction> {
        match key.code {
            KeyCode::Esc => {
                self.mode = VimMode::Normal;
                self.search_pattern.clear();
                Ok(VimAction::ModeChange(VimMode::Normal))
            }
            KeyCode::Enter => {
                let pattern = self.search_pattern.clone();
                self.search_history.push(pattern.clone());
                self.mode = VimMode::Normal;
                let direction = if self.search_backward {
                    super::SearchDirection::Backward
                } else {
                    super::SearchDirection::Forward
                };
                Ok(VimAction::Search(direction, pattern))
            }
            KeyCode::Char(c) => {
                self.search_pattern.push(c);
                Ok(VimAction::None)
            }
            KeyCode::Backspace => {
                self.search_pattern.pop();
                Ok(VimAction::None)
            }
            _ => Ok(VimAction::None),
        }
    }

    pub fn handle_operator_key(&mut self, key: KeyEvent) -> Result<VimAction> {
        if let Some(operator) = &self.operator {
            let action = match operator {
                Operator::Mark => {
                    if let KeyCode::Char(c) = key.code {
                        // Set mark at current position
                        // TODO: Implement mark setting
                        VimAction::None
                    } else {
                        VimAction::None
                    }
                }
                Operator::JumpToMark => {
                    if let KeyCode::Char(c) = key.code {
                        // TODO: Implement jump to mark
                        VimAction::None
                    } else {
                        VimAction::None
                    }
                }
                _ => {
                    // Handle motion after operator
                    // For now, just cancel
                    VimAction::None
                }
            };

            self.mode = VimMode::Normal;
            self.operator = None;
            Ok(action)
        } else {
            self.mode = VimMode::Normal;
            Ok(VimAction::None)
        }
    }

    fn get_selection(&self) -> SelectionRange {
        SelectionRange {
            start: self.visual_anchor.unwrap_or(self.cursor),
            end: self.cursor,
            mode: self.visual_mode_type.clone().unwrap_or(SelectionMode::Character),
        }
    }

    pub fn update_cursor(&mut self, pos: Position) {
        self.cursor = pos;
    }
}