use std::collections::HashMap;

/// Lane type with priority hints
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneType {
    MainTrunk,      // Primary branch (e.g., main/master) - highest priority
    ActiveBranch,   // Currently checked out branch
    FeatureBranch,  // Regular feature branch
    ReleaseBranch,  // Release/hotfix branch
    RemoteBranch,   // Remote tracking branch
    Detached,       // Detached HEAD or orphan
}

impl LaneType {
    /// Get base priority for this lane type (higher = more important)
    pub fn priority(&self) -> u8 {
        match self {
            LaneType::MainTrunk => 20,
            LaneType::ActiveBranch => 18,
            LaneType::ReleaseBranch => 15,
            LaneType::FeatureBranch => 10,
            LaneType::RemoteBranch => 8,
            LaneType::Detached => 5,
        }
    }
}

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

    /// Route a cell with multiple potential paths and lane priorities
    pub fn route_cell_with_priority(
        &self,
        entries: &[(EntryDir, LaneType)],
        exits: &[(ExitDir, LaneType)],
        prefer_straight: bool,
    ) -> char {
        let mut best_char = ' ';
        let mut best_score = 0u32;

        // Calculate scores for each combination
        for (entry_dir, entry_lane) in entries {
            for (exit_dir, exit_lane) in exits {
                if let Some(decision) = self.char_table.get(&(*entry_dir, *exit_dir)) {
                    // Score = base priority + lane priority bonus + straight line bonus
                    let mut score = decision.priority as u32 * 100;

                    // Add lane type priorities
                    score += entry_lane.priority() as u32 * 10;
                    score += exit_lane.priority() as u32 * 10;

                    // Bonus for maintaining straight lines on main trunk
                    if prefer_straight && entry_lane == exit_lane {
                        if *entry_dir == EntryDir::North && *exit_dir == ExitDir::South {
                            score += 50; // Vertical straight bonus
                        } else if *entry_dir == EntryDir::West && *exit_dir == ExitDir::East {
                            score += 50; // Horizontal straight bonus
                        }
                    }

                    // Main trunk always gets straight path priority
                    if *entry_lane == LaneType::MainTrunk && *exit_lane == LaneType::MainTrunk {
                        score += 100;
                    }

                    if score > best_score {
                        best_score = score;
                        best_char = decision.character;
                    }
                }
            }
        }

        // Enhanced fallback logic
        if best_char == ' ' {
            best_char = self.select_fallback_char(entries, exits);
        }

        best_char
    }

    /// Select fallback character when no exact match found
    fn select_fallback_char(
        &self,
        entries: &[(EntryDir, LaneType)],
        exits: &[(ExitDir, LaneType)],
    ) -> char {
        // Check if we have main trunk
        let has_main_trunk = entries.iter().any(|(_, lt)| *lt == LaneType::MainTrunk)
            || exits.iter().any(|(_, lt)| *lt == LaneType::MainTrunk);

        // Count directions
        let has_north = entries.iter().any(|(d, _)| *d == EntryDir::North);
        let has_south = exits.iter().any(|(d, _)| *d == ExitDir::South);
        let has_east = exits.iter().any(|(d, _)| *d == ExitDir::East);
        let has_west = entries.iter().any(|(d, _)| *d == EntryDir::West);

        // Decision matrix
        match (has_north || has_south, has_east || has_west, has_main_trunk) {
            (true, true, true) => {
                // Main trunk at junction - prefer T-junction over cross
                match self.profile {
                    CharsetProfile::Utf8Rounded | CharsetProfile::Utf8Straight => '├',
                    CharsetProfile::Ascii => '+',
                }
            }
            (true, true, false) => {
                // Regular cross
                match self.profile {
                    CharsetProfile::Utf8Rounded | CharsetProfile::Utf8Straight => '┼',
                    CharsetProfile::Ascii => '+',
                }
            }
            (true, false, _) => {
                // Vertical line
                match self.profile {
                    CharsetProfile::Utf8Rounded | CharsetProfile::Utf8Straight => '│',
                    CharsetProfile::Ascii => '|',
                }
            }
            (false, true, _) => {
                // Horizontal line
                match self.profile {
                    CharsetProfile::Utf8Rounded | CharsetProfile::Utf8Straight => '─',
                    CharsetProfile::Ascii => '-',
                }
            }
            _ => ' ',
        }
    }

    /// Route a cell with multiple potential paths (compatibility method)
    pub fn route_cell(&self, entries: &[EntryDir], exits: &[ExitDir]) -> char {
        // Convert to new format with default lane type
        let entries_with_lane: Vec<(EntryDir, LaneType)> = entries
            .iter()
            .map(|&dir| (dir, LaneType::FeatureBranch))
            .collect();
        let exits_with_lane: Vec<(ExitDir, LaneType)> = exits
            .iter()
            .map(|&dir| (dir, LaneType::FeatureBranch))
            .collect();

        self.route_cell_with_priority(&entries_with_lane, &exits_with_lane, false)
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