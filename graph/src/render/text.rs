use unicode_width::UnicodeWidthStr;
use unicode_segmentation::UnicodeSegmentation;

/// CJK mode for width calculation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CjkMode {
    Auto,  // Detect from locale
    On,    // Force CJK width
    Off,   // Western width only
}

/// Text layout and width calculation
pub struct TextLayout {
    cjk_mode: CjkMode,
}

impl TextLayout {
    pub fn new(cjk_mode: CjkMode) -> Self {
        Self { cjk_mode }
    }

    /// Calculate display width of a string
    pub fn display_width(&self, text: &str) -> usize {
        match self.cjk_mode {
            CjkMode::On | CjkMode::Auto => {
                // Use unicode-width with CJK rules
                UnicodeWidthStr::width(text)
            }
            CjkMode::Off => {
                // Count grapheme clusters
                text.graphemes(true).count()
            }
        }
    }

    /// Truncate string to fit width, preserving grapheme boundaries
    pub fn truncate_to_width(&self, text: &str, max_width: usize) -> String {
        if max_width == 0 {
            return String::new();
        }

        let mut result = String::new();
        let mut current_width = 0;

        for grapheme in text.graphemes(true) {
            let grapheme_width = if self.cjk_mode != CjkMode::Off {
                UnicodeWidthStr::width(grapheme)
            } else {
                1
            };

            if current_width + grapheme_width > max_width {
                // Add ellipsis if there's room
                if current_width + 1 <= max_width {
                    result.push('…');
                } else if current_width + 3 <= max_width {
                    result.push_str("...");
                }
                break;
            }

            result.push_str(grapheme);
            current_width += grapheme_width;
        }

        result
    }

    /// Pad string to exact width (for alignment)
    pub fn pad_to_width(&self, text: &str, target_width: usize, align: Alignment) -> String {
        let text_width = self.display_width(text);

        if text_width >= target_width {
            return self.truncate_to_width(text, target_width);
        }

        let padding = target_width - text_width;

        match align {
            Alignment::Left => format!("{}{}", text, " ".repeat(padding)),
            Alignment::Right => format!("{}{}", " ".repeat(padding), text),
            Alignment::Center => {
                let left_pad = padding / 2;
                let right_pad = padding - left_pad;
                format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
            }
        }
    }

    /// Split text into lines of maximum width
    pub fn wrap(&self, text: &str, max_width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut current_width = 0;

        for word in text.split_whitespace() {
            let word_width = self.display_width(word);

            if current_width > 0 && current_width + 1 + word_width > max_width {
                // Start new line
                lines.push(current_line);
                current_line = word.to_string();
                current_width = word_width;
            } else {
                if !current_line.is_empty() {
                    current_line.push(' ');
                    current_width += 1;
                }
                current_line.push_str(word);
                current_width += word_width;
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }

    /// Detect if string contains CJK characters
    pub fn has_cjk(text: &str) -> bool {
        text.chars().any(|ch| {
            matches!(ch,
                '\u{4E00}'..='\u{9FFF}' | // CJK Unified Ideographs
                '\u{3400}'..='\u{4DBF}' | // CJK Extension A
                '\u{F900}'..='\u{FAFF}' | // CJK Compatibility
                '\u{3040}'..='\u{309F}' | // Hiragana
                '\u{30A0}'..='\u{30FF}' | // Katakana
                '\u{AC00}'..='\u{D7AF}'   // Hangul
            )
        })
    }

    /// Auto-detect CJK mode from locale
    pub fn detect_cjk_from_locale() -> bool {
        if let Ok(lang) = std::env::var("LANG") {
            lang.contains("zh") || lang.contains("ja") || lang.contains("ko")
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

/// Special handling for mixed CJK/ASCII in commit messages
pub struct CommitMessageFormatter {
    layout: TextLayout,
}

impl CommitMessageFormatter {
    pub fn new(cjk_mode: CjkMode) -> Self {
        Self {
            layout: TextLayout::new(cjk_mode),
        }
    }

    /// Format commit message with proper alignment
    pub fn format(&self, sha: &str, message: &str, max_width: usize) -> String {
        // Reserve space for SHA (8 chars) + space
        let sha_width = 9;
        let message_width = max_width.saturating_sub(sha_width);

        let truncated_message = self.layout.truncate_to_width(message, message_width);

        format!("{} {}", &sha[..8.min(sha.len())], truncated_message)
    }

    /// Format branch name with CJK support
    pub fn format_branch(&self, branch: &str, max_width: usize) -> String {
        if branch.len() <= max_width {
            branch.to_string()
        } else {
            // Smart truncation: keep important parts
            if branch.contains('/') {
                let parts: Vec<&str> = branch.split('/').collect();
                if parts.len() >= 2 {
                    let prefix = &parts[0][..1.min(parts[0].len())];
                    let suffix = parts.last().unwrap();
                    let truncated = self.layout.truncate_to_width(suffix, max_width - 2);
                    format!("{}/{}", prefix, truncated)
                } else {
                    self.layout.truncate_to_width(branch, max_width)
                }
            } else {
                self.layout.truncate_to_width(branch, max_width)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cjk_width() {
        let layout = TextLayout::new(CjkMode::On);

        // ASCII
        assert_eq!(layout.display_width("hello"), 5);

        // CJK (each character is width 2)
        assert_eq!(layout.display_width("你好"), 4);

        // Mixed
        assert_eq!(layout.display_width("hello世界"), 9);
    }

    #[test]
    fn test_truncate_cjk() {
        let layout = TextLayout::new(CjkMode::On);

        let text = "Hello世界World";
        let truncated = layout.truncate_to_width(text, 10);
        assert!(layout.display_width(&truncated) <= 10);

        // Should not break grapheme
        let emoji_text = "Hi👨‍👩‍👧‍👦there";
        let truncated = layout.truncate_to_width(emoji_text, 5);
        assert!(truncated.starts_with("Hi"));
    }

    #[test]
    fn test_padding() {
        let layout = TextLayout::new(CjkMode::On);

        let padded = layout.pad_to_width("测试", 10, Alignment::Left);
        assert_eq!(layout.display_width(&padded), 10);

        let padded = layout.pad_to_width("test", 10, Alignment::Center);
        assert_eq!(layout.display_width(&padded), 10);
        assert!(padded.contains("test"));
    }

    #[test]
    fn test_has_cjk() {
        assert!(TextLayout::has_cjk("包含中文"));
        assert!(TextLayout::has_cjk("日本語"));
        assert!(TextLayout::has_cjk("한글"));
        assert!(!TextLayout::has_cjk("English only"));
    }

    #[test]
    fn test_commit_formatter() {
        let formatter = CommitMessageFormatter::new(CjkMode::On);

        let formatted = formatter.format(
            "abc123def",
            "feat: 实现中文支持功能",
            30,
        );
        assert!(formatted.starts_with("abc123de"));
        assert!(formatted.contains("实现"));
    }
}