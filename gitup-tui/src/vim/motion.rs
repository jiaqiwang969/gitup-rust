use super::Position;

/// Motion types for Vim navigation
#[derive(Debug, Clone)]
pub enum Motion {
    // Character motions
    Left(usize),
    Right(usize),
    Up(usize),
    Down(usize),

    // Word motions
    WordForward(usize),
    WordBackward(usize),
    WordEnd(usize),
    WORDForward(usize),  // WORD = space-delimited
    WORDBackward(usize),
    WORDEnd(usize),

    // Line motions
    LineStart,
    LineEnd,
    LineFirstNonBlank,
    Line(usize), // Go to specific line

    // Paragraph/Section motions
    ParagraphForward(usize),
    ParagraphBackward(usize),
    SectionForward(usize),
    SectionBackward(usize),

    // File motions
    FileStart,
    FileEnd,

    // Page motions
    PageDown,
    PageUp,
    HalfPageDown,
    HalfPageUp,

    // Search motions
    FindChar(char, usize),
    FindCharBackward(char, usize),
    TillChar(char, usize),
    TillCharBackward(char, usize),
    RepeatFind,
    RepeatFindReverse,

    // Git-specific motions
    NextCommit(usize),
    PrevCommit(usize),
    NextBranch(usize),
    PrevBranch(usize),
    NextMerge(usize),
    PrevMerge(usize),
    NextConflict(usize),
    PrevConflict(usize),
    ParentCommit(usize),
    ChildCommit(usize),

    // Text object motions
    InnerWord,
    AroundWord,
    InnerParagraph,
    AroundParagraph,
    InnerBlock(char), // {, [, (, <, etc.
    AroundBlock(char),

    // Git text objects
    InnerCommit,
    AroundCommit,
    InnerBranch,
    AroundBranch,
    InnerHunk,
    AroundHunk,
}

/// Type of motion for determining operation scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionType {
    /// Character-wise motion
    CharWise,

    /// Line-wise motion
    LineWise,

    /// Block-wise motion
    BlockWise,
}

impl Motion {
    /// Get the type of this motion
    pub fn motion_type(&self) -> MotionType {
        match self {
            // Line-wise motions
            Motion::Up(_) | Motion::Down(_) |
            Motion::ParagraphForward(_) | Motion::ParagraphBackward(_) |
            Motion::NextCommit(_) | Motion::PrevCommit(_) => MotionType::LineWise,

            // Character-wise motions
            _ => MotionType::CharWise,
        }
    }

    /// Apply motion from a position and return the new position
    pub fn apply(&self, from: Position, context: &dyn MotionContext) -> Position {
        match self {
            Motion::Left(n) => Position::new(from.row, from.col.saturating_sub(*n)),
            Motion::Right(n) => Position::new(from.row, from.col + n),
            Motion::Up(n) => Position::new(from.row.saturating_sub(*n), from.col),
            Motion::Down(n) => Position::new(from.row + n, from.col),

            Motion::LineStart => Position::new(from.row, 0),
            Motion::LineEnd => Position::new(from.row, context.line_length(from.row)),
            Motion::LineFirstNonBlank => {
                Position::new(from.row, context.first_non_blank(from.row))
            }

            Motion::FileStart => Position::new(0, 0),
            Motion::FileEnd => Position::new(context.total_lines().saturating_sub(1), 0),

            Motion::Line(n) => Position::new(n.saturating_sub(1), from.col),

            Motion::WordForward(n) => {
                let mut pos = from;
                for _ in 0..*n {
                    pos = context.next_word_start(pos);
                }
                pos
            }
            Motion::WordBackward(n) => {
                let mut pos = from;
                for _ in 0..*n {
                    pos = context.prev_word_start(pos);
                }
                pos
            }
            Motion::WordEnd(n) => {
                let mut pos = from;
                for _ in 0..*n {
                    pos = context.next_word_end(pos);
                }
                pos
            }

            Motion::NextCommit(n) => {
                let mut pos = from;
                for _ in 0..*n {
                    if let Some(next) = context.next_commit(pos) {
                        pos = next;
                    } else {
                        break;
                    }
                }
                pos
            }
            Motion::PrevCommit(n) => {
                let mut pos = from;
                for _ in 0..*n {
                    if let Some(prev) = context.prev_commit(pos) {
                        pos = prev;
                    } else {
                        break;
                    }
                }
                pos
            }

            Motion::ParentCommit(n) => {
                if let Some(parent) = context.parent_commit(from, *n) {
                    parent
                } else {
                    from
                }
            }
            Motion::ChildCommit(n) => {
                if let Some(child) = context.child_commit(from, *n) {
                    child
                } else {
                    from
                }
            }

            _ => from, // TODO: Implement remaining motions
        }
    }

    /// Get the range covered by this motion from a position
    pub fn get_range(&self, from: Position, context: &dyn MotionContext) -> (Position, Position) {
        let to = self.apply(from, context);

        // Ensure start <= end
        if from.row < to.row || (from.row == to.row && from.col <= to.col) {
            (from, to)
        } else {
            (to, from)
        }
    }

    /// Check if this is a Git-specific motion
    pub fn is_git_motion(&self) -> bool {
        matches!(
            self,
            Motion::NextCommit(_) | Motion::PrevCommit(_) |
            Motion::NextBranch(_) | Motion::PrevBranch(_) |
            Motion::NextMerge(_) | Motion::PrevMerge(_) |
            Motion::NextConflict(_) | Motion::PrevConflict(_) |
            Motion::ParentCommit(_) | Motion::ChildCommit(_) |
            Motion::InnerCommit | Motion::AroundCommit |
            Motion::InnerBranch | Motion::AroundBranch |
            Motion::InnerHunk | Motion::AroundHunk
        )
    }
}

/// Context for applying motions (provided by the graph view)
pub trait MotionContext {
    /// Get the length of a line
    fn line_length(&self, row: usize) -> usize;

    /// Get the first non-blank column in a line
    fn first_non_blank(&self, row: usize) -> usize;

    /// Get the total number of lines
    fn total_lines(&self) -> usize;

    /// Find the next word start position
    fn next_word_start(&self, from: Position) -> Position;

    /// Find the previous word start position
    fn prev_word_start(&self, from: Position) -> Position;

    /// Find the next word end position
    fn next_word_end(&self, from: Position) -> Position;

    /// Find the next commit position
    fn next_commit(&self, from: Position) -> Option<Position>;

    /// Find the previous commit position
    fn prev_commit(&self, from: Position) -> Option<Position>;

    /// Find the parent commit position
    fn parent_commit(&self, from: Position, n: usize) -> Option<Position>;

    /// Find the child commit position
    fn child_commit(&self, from: Position, n: usize) -> Option<Position>;
}