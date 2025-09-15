use super::Position;

/// Git-specific text objects for Vim operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitTextObject {
    // Commit-related
    InnerCommit,    // ic - commit message only
    AroundCommit,   // ac - entire commit with metadata

    // Branch-related
    InnerBranch,    // ib - commits unique to branch
    AroundBranch,   // ab - all commits in branch

    // Hunk-related (for diffs)
    InnerHunk,      // ih - changed lines only
    AroundHunk,     // ah - hunk with context

    // Message-related
    InnerMessage,   // im - commit message body
    AroundMessage,  // am - commit message with subject

    // Author-related
    InnerAuthor,    // ia - commits by same author
    AroundAuthor,   // aa - author block with context

    // Date-related
    InnerDate,      // id - commits on same date
    AroundDate,     // ad - date range

    // File-related
    InnerFile,      // if - changes to specific file
    AroundFile,     // af - file with related commits

    // Range-related
    InnerRange,     // ir - commit range
    AroundRange,    // ar - range with merge base

    // Conflict-related
    InnerConflict,  // ix - conflict content
    AroundConflict, // ax - conflict with markers
}

impl GitTextObject {
    /// Get the text object from character pairs (e.g., "ic", "ab")
    pub fn from_chars(modifier: char, object: char) -> Option<Self> {
        match (modifier, object) {
            ('i', 'c') => Some(GitTextObject::InnerCommit),
            ('a', 'c') => Some(GitTextObject::AroundCommit),

            ('i', 'b') => Some(GitTextObject::InnerBranch),
            ('a', 'b') => Some(GitTextObject::AroundBranch),

            ('i', 'h') => Some(GitTextObject::InnerHunk),
            ('a', 'h') => Some(GitTextObject::AroundHunk),

            ('i', 'm') => Some(GitTextObject::InnerMessage),
            ('a', 'm') => Some(GitTextObject::AroundMessage),

            ('i', 'a') => Some(GitTextObject::InnerAuthor),
            ('a', 'a') => Some(GitTextObject::AroundAuthor),

            ('i', 'd') => Some(GitTextObject::InnerDate),
            ('a', 'd') => Some(GitTextObject::AroundDate),

            ('i', 'f') => Some(GitTextObject::InnerFile),
            ('a', 'f') => Some(GitTextObject::AroundFile),

            ('i', 'r') => Some(GitTextObject::InnerRange),
            ('a', 'r') => Some(GitTextObject::AroundRange),

            ('i', 'x') => Some(GitTextObject::InnerConflict),
            ('a', 'x') => Some(GitTextObject::AroundConflict),

            _ => None,
        }
    }

    /// Check if this is an "inner" text object
    pub fn is_inner(&self) -> bool {
        matches!(
            self,
            GitTextObject::InnerCommit
                | GitTextObject::InnerBranch
                | GitTextObject::InnerHunk
                | GitTextObject::InnerMessage
                | GitTextObject::InnerAuthor
                | GitTextObject::InnerDate
                | GitTextObject::InnerFile
                | GitTextObject::InnerRange
                | GitTextObject::InnerConflict
        )
    }

    /// Check if this is an "around" text object
    pub fn is_around(&self) -> bool {
        !self.is_inner()
    }

    /// Get description for help text
    pub fn description(&self) -> &'static str {
        match self {
            GitTextObject::InnerCommit => "inside commit (message only)",
            GitTextObject::AroundCommit => "around commit (with metadata)",
            GitTextObject::InnerBranch => "inside branch (unique commits)",
            GitTextObject::AroundBranch => "around branch (all commits)",
            GitTextObject::InnerHunk => "inside hunk (changes only)",
            GitTextObject::AroundHunk => "around hunk (with context)",
            GitTextObject::InnerMessage => "inside message (body only)",
            GitTextObject::AroundMessage => "around message (with subject)",
            GitTextObject::InnerAuthor => "inside author (same author)",
            GitTextObject::AroundAuthor => "around author (with context)",
            GitTextObject::InnerDate => "inside date (same date)",
            GitTextObject::AroundDate => "around date (date range)",
            GitTextObject::InnerFile => "inside file (file changes)",
            GitTextObject::AroundFile => "around file (with commits)",
            GitTextObject::InnerRange => "inside range (commits only)",
            GitTextObject::AroundRange => "around range (with base)",
            GitTextObject::InnerConflict => "inside conflict (content)",
            GitTextObject::AroundConflict => "around conflict (with markers)",
        }
    }
}

/// Context for evaluating text objects in the Git graph
pub trait GitTextObjectContext {
    /// Get the range for a commit text object
    fn get_commit_range(&self, pos: Position, around: bool) -> (Position, Position);

    /// Get the range for a branch text object
    fn get_branch_range(&self, pos: Position, around: bool) -> (Position, Position);

    /// Get the range for a hunk text object
    fn get_hunk_range(&self, pos: Position, around: bool) -> (Position, Position);

    /// Get the range for a message text object
    fn get_message_range(&self, pos: Position, around: bool) -> (Position, Position);

    /// Get the range for an author text object
    fn get_author_range(&self, pos: Position, around: bool) -> (Position, Position);

    /// Get the range for a date text object
    fn get_date_range(&self, pos: Position, around: bool) -> (Position, Position);

    /// Get the range for a file text object
    fn get_file_range(&self, pos: Position, around: bool) -> (Position, Position);

    /// Get the range for a range text object
    fn get_range_range(&self, pos: Position, around: bool) -> (Position, Position);

    /// Get the range for a conflict text object
    fn get_conflict_range(&self, pos: Position, around: bool) -> (Position, Position);
}

/// Standard Vim text objects (for completeness)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardTextObject {
    // Word objects
    InnerWord,      // iw
    AroundWord,     // aw
    InnerWORD,      // iW
    AroundWORD,     // aW

    // Sentence/Paragraph
    InnerSentence,  // is
    AroundSentence, // as
    InnerParagraph, // ip
    AroundParagraph,// ap

    // Block objects
    InnerParen,     // i(, i)
    AroundParen,    // a(, a)
    InnerBracket,   // i[, i]
    AroundBracket,  // a[, a]
    InnerBrace,     // i{, i}
    AroundBrace,    // a{, a}
    InnerAngle,     // i<, i>
    AroundAngle,    // a<, a>

    // Quote objects
    InnerSingleQuote,  // i'
    AroundSingleQuote, // a'
    InnerDoubleQuote,  // i"
    AroundDoubleQuote, // a"
    InnerBacktick,     // i`
    AroundBacktick,    // a`

    // Tag objects (for XML/HTML)
    InnerTag,       // it
    AroundTag,      // at
}

impl StandardTextObject {
    /// Get the text object from character pairs
    pub fn from_chars(modifier: char, object: char) -> Option<Self> {
        match (modifier, object) {
            ('i', 'w') => Some(StandardTextObject::InnerWord),
            ('a', 'w') => Some(StandardTextObject::AroundWord),
            ('i', 'W') => Some(StandardTextObject::InnerWORD),
            ('a', 'W') => Some(StandardTextObject::AroundWORD),

            ('i', 's') => Some(StandardTextObject::InnerSentence),
            ('a', 's') => Some(StandardTextObject::AroundSentence),
            ('i', 'p') => Some(StandardTextObject::InnerParagraph),
            ('a', 'p') => Some(StandardTextObject::AroundParagraph),

            ('i', '(') | ('i', ')') => Some(StandardTextObject::InnerParen),
            ('a', '(') | ('a', ')') => Some(StandardTextObject::AroundParen),
            ('i', '[') | ('i', ']') => Some(StandardTextObject::InnerBracket),
            ('a', '[') | ('a', ']') => Some(StandardTextObject::AroundBracket),
            ('i', '{') | ('i', '}') => Some(StandardTextObject::InnerBrace),
            ('a', '{') | ('a', '}') => Some(StandardTextObject::AroundBrace),
            ('i', '<') | ('i', '>') => Some(StandardTextObject::InnerAngle),
            ('a', '<') | ('a', '>') => Some(StandardTextObject::AroundAngle),

            ('i', '\'') => Some(StandardTextObject::InnerSingleQuote),
            ('a', '\'') => Some(StandardTextObject::AroundSingleQuote),
            ('i', '"') => Some(StandardTextObject::InnerDoubleQuote),
            ('a', '"') => Some(StandardTextObject::AroundDoubleQuote),
            ('i', '`') => Some(StandardTextObject::InnerBacktick),
            ('a', '`') => Some(StandardTextObject::AroundBacktick),

            ('i', 't') => Some(StandardTextObject::InnerTag),
            ('a', 't') => Some(StandardTextObject::AroundTag),

            _ => None,
        }
    }
}