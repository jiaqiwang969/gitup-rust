use std::collections::HashMap;

/// Direction of line entering a cell
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryDir {
    None,
    North,      // From above
    South,      // From below
    East,       // From right
    West,       // From left
    NorthEast,  // Diagonal from top-right
    NorthWest,  // Diagonal from top-left
    SouthEast,  // Diagonal from bottom-right
    SouthWest,  // Diagonal from bottom-left
}

/// Direction of line exiting a cell
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExitDir {
    None,
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

/// Character set profile
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CharsetProfile {
    Utf8Rounded,  // Default with rounded corners
    Utf8Straight, // Sharp corners
    Ascii,        // ASCII fallback
}

/// Cell routing decision
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub character: char,
    pub priority: u8,
    pub fallback: Option<char>,
}

/// Edge router with priority-based character selection
pub struct CellRouter {
    /// Character lookup table based on entry/exit combinations
    char_table: HashMap<(EntryDir, ExitDir), RoutingDecision>,
    /// Current charset profile
    profile: CharsetProfile,
}

impl CellRouter {
    pub fn new(profile: CharsetProfile) -> Self {
        let mut router = Self {
            char_table: HashMap::new(),
            profile,
        };
        router.build_char_table();
        router
    }

    /// Build character lookup table based on profile
    fn build_char_table(&mut self) {
        match self.profile {
            CharsetProfile::Utf8Rounded => {
                // Straight lines
                self.insert(EntryDir::North, ExitDir::South, '│', 10, Some('|'));
                self.insert(EntryDir::East, ExitDir::West, '─', 10, Some('-'));

                // Corners
                self.insert(EntryDir::North, ExitDir::East, '╰', 8, Some('\\'));
                self.insert(EntryDir::North, ExitDir::West, '╯', 8, Some('/'));
                self.insert(EntryDir::South, ExitDir::East, '╭', 8, Some('/'));
                self.insert(EntryDir::South, ExitDir::West, '╮', 8, Some('\\'));

                // T-junctions (merges have higher priority)
                self.insert(EntryDir::North, ExitDir::None, '╵', 9, Some('|'));
                self.insert(EntryDir::South, ExitDir::None, '╷', 9, Some('|'));
                self.insert(EntryDir::East, ExitDir::None, '╴', 9, Some('-'));
                self.insert(EntryDir::West, ExitDir::None, '╶', 9, Some('-'));

                // Merge patterns (highest priority)
                self.insert(EntryDir::NorthWest, ExitDir::South, '┤', 12, Some('|'));
                self.insert(EntryDir::NorthEast, ExitDir::South, '├', 12, Some('|'));
                self.insert(EntryDir::North, ExitDir::SouthWest, '┤', 12, Some('|'));
                self.insert(EntryDir::North, ExitDir::SouthEast, '├', 12, Some('|'));

                // Cross
                self.insert(EntryDir::North, ExitDir::East, '├', 11, Some('+'));
                self.insert(EntryDir::North, ExitDir::West, '┤', 11, Some('+'));
                self.insert(EntryDir::South, ExitDir::East, '├', 11, Some('+'));
                self.insert(EntryDir::South, ExitDir::West, '┤', 11, Some('+'));
            }

            CharsetProfile::Utf8Straight => {
                // Straight lines
                self.insert(EntryDir::North, ExitDir::South, '│', 10, Some('|'));
                self.insert(EntryDir::East, ExitDir::West, '─', 10, Some('-'));

                // Sharp corners
                self.insert(EntryDir::North, ExitDir::East, '└', 8, Some('\\'));
                self.insert(EntryDir::North, ExitDir::West, '┘', 8, Some('/'));
                self.insert(EntryDir::South, ExitDir::East, '┌', 8, Some('/'));
                self.insert(EntryDir::South, ExitDir::West, '┐', 8, Some('\\'));

                // T-junctions
                self.insert(EntryDir::North, ExitDir::None, '╵', 9, Some('|'));
                self.insert(EntryDir::South, ExitDir::None, '╷', 9, Some('|'));

                // Merge patterns
                self.insert(EntryDir::NorthWest, ExitDir::South, '┤', 12, Some('|'));
                self.insert(EntryDir::NorthEast, ExitDir::South, '├', 12, Some('|'));

                // Cross
                self.insert(EntryDir::North, ExitDir::South, '│', 10, Some('|'));
                self.insert(EntryDir::East, ExitDir::West, '─', 10, Some('-'));
                self.insert_cross('┼', 11, Some('+'));
            }

            CharsetProfile::Ascii => {
                // ASCII only
                self.insert(EntryDir::North, ExitDir::South, '|', 10, None);
                self.insert(EntryDir::East, ExitDir::West, '-', 10, None);
                self.insert(EntryDir::North, ExitDir::East, '\\', 8, None);
                self.insert(EntryDir::North, ExitDir::West, '/', 8, None);
                self.insert(EntryDir::South, ExitDir::East, '/', 8, None);
                self.insert(EntryDir::South, ExitDir::West, '\\', 8, None);
                self.insert_cross('+', 11, None);
            }
        }
    }

    fn insert(&mut self, entry: EntryDir, exit: ExitDir, ch: char, priority: u8, fallback: Option<char>) {
        self.char_table.insert(
            (entry, exit),
            RoutingDecision {
                character: ch,
                priority,
                fallback,
            },
        );
    }

    fn insert_cross(&mut self, ch: char, priority: u8, fallback: Option<char>) {
        // Helper for cross patterns
        self.insert(EntryDir::North, ExitDir::South, ch, priority, fallback);
        self.insert(EntryDir::East, ExitDir::West, ch, priority, fallback);
        self.insert(EntryDir::North, ExitDir::East, ch, priority, fallback);
        self.insert(EntryDir::North, ExitDir::West, ch, priority, fallback);
        self.insert(EntryDir::South, ExitDir::East, ch, priority, fallback);
        self.insert(EntryDir::South, ExitDir::West, ch, priority, fallback);
    }

    /// Route a cell with multiple potential paths
    pub fn route_cell(&self, entries: &[EntryDir], exits: &[ExitDir]) -> char {
        let mut best_char = ' ';
        let mut best_priority = 0u8;

        // Try all combinations
        for entry in entries {
            for exit in exits {
                if let Some(decision) = self.char_table.get(&(*entry, *exit)) {
                    if decision.priority > best_priority {
                        best_priority = decision.priority;
                        best_char = decision.character;
                    }
                }
            }
        }

        // Fallback logic for common patterns
        if best_char == ' ' {
            if !entries.is_empty() && !exits.is_empty() {
                // Default to cross if multiple paths
                best_char = match self.profile {
                    CharsetProfile::Utf8Rounded | CharsetProfile::Utf8Straight => '┼',
                    CharsetProfile::Ascii => '+',
                };
            } else if entries.contains(&EntryDir::North) || exits.contains(&ExitDir::South) {
                // Vertical line
                best_char = match self.profile {
                    CharsetProfile::Utf8Rounded | CharsetProfile::Utf8Straight => '│',
                    CharsetProfile::Ascii => '|',
                };
            } else if entries.contains(&EntryDir::East) || exits.contains(&ExitDir::West) {
                // Horizontal line
                best_char = match self.profile {
                    CharsetProfile::Utf8Rounded | CharsetProfile::Utf8Straight => '─',
                    CharsetProfile::Ascii => '-',
                };
            }
        }

        best_char
    }

    /// Get fallback character for a given character
    pub fn get_fallback(&self, ch: char) -> char {
        for decision in self.char_table.values() {
            if decision.character == ch {
                return decision.fallback.unwrap_or(ch);
            }
        }
        ch
    }
}

/// Conflict resolution for overlapping edges
pub struct ConflictResolver {
    router: CellRouter,
}

impl ConflictResolver {
    pub fn new(profile: CharsetProfile) -> Self {
        Self {
            router: CellRouter::new(profile),
        }
    }

    /// Resolve character for a cell with multiple competing edges
    pub fn resolve(&self, candidates: &[(char, u8)]) -> char {
        if candidates.is_empty() {
            return ' ';
        }

        // Sort by priority and pick highest
        let mut sorted = candidates.to_vec();
        sorted.sort_by_key(|&(_, priority)| std::cmp::Reverse(priority));

        sorted[0].0
    }

    /// Merge two characters into one
    pub fn merge_chars(&self, primary: char, secondary: char) -> char {
        // Priority-based merging
        match (primary, secondary) {
            ('│', '─') | ('─', '│') => '┼',
            ('│', _) => primary,
            (_, '│') => secondary,
            ('─', _) => primary,
            (_, '─') => secondary,
            _ => primary,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_basic() {
        let router = CellRouter::new(CharsetProfile::Utf8Straight);

        // Vertical line
        let ch = router.route_cell(&[EntryDir::North], &[ExitDir::South]);
        assert_eq!(ch, '│');

        // Corner
        let ch = router.route_cell(&[EntryDir::North], &[ExitDir::East]);
        assert_eq!(ch, '└');
    }

    #[test]
    fn test_router_merge() {
        let router = CellRouter::new(CharsetProfile::Utf8Straight);

        // Merge pattern
        let ch = router.route_cell(
            &[EntryDir::NorthWest, EntryDir::NorthEast],
            &[ExitDir::South],
        );
        // Should prioritize merge character
        assert!(ch == '┤' || ch == '├' || ch == '┼');
    }

    #[test]
    fn test_ascii_fallback() {
        let router = CellRouter::new(CharsetProfile::Ascii);

        let ch = router.route_cell(&[EntryDir::North], &[ExitDir::South]);
        assert_eq!(ch, '|');

        let ch = router.route_cell(&[EntryDir::North], &[ExitDir::East]);
        assert_eq!(ch, '\\');
    }

    #[test]
    fn test_conflict_resolver() {
        let resolver = ConflictResolver::new(CharsetProfile::Utf8Straight);

        let ch = resolver.resolve(&[('│', 10), ('─', 8), ('┤', 12)]);
        assert_eq!(ch, '┤'); // Highest priority

        let merged = resolver.merge_chars('│', '─');
        assert_eq!(merged, '┼');
    }
}