use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Check if a character is East Asian ambiguous width
fn is_east_asian_ambiguous(s: &str) -> bool {
    // Common EA ambiguous characters that should be width 2 in CJK context
    s.chars().any(|c| matches!(c,
        '\u{00A1}'..='\u{00A9}' | // Latin supplement symbols
        '\u{2010}'..='\u{2027}' | // Punctuation
        '\u{2030}'..='\u{203E}' | // General punctuation
        '\u{2103}'..='\u{2126}' | // Letterlike symbols
        '\u{2190}'..='\u{2199}' | // Arrows
        '\u{2460}'..='\u{24FF}' | // Enclosed alphanumerics
        '\u{25A0}'..='\u{25FF}' | // Geometric shapes
        '\u{2600}'..='\u{26FF}' | // Miscellaneous symbols
        '\u{2E80}'..='\u{9FFF}' | // CJK range
        '\u{F900}'..='\u{FAFF}' | // CJK compatibility
        '\u{FE30}'..='\u{FE4F}' | // CJK compatibility forms
        '\u{FF01}'..='\u{FF60}' | // Halfwidth and fullwidth forms
        '\u{FFE0}'..='\u{FFE6}'   // Fullwidth symbols
    ))
}

/// Calculate display width for a string, considering CJK context
pub fn display_width(s: &str, cjk_context: bool) -> usize {
    let mut width = 0;
    for grapheme in s.graphemes(true) {
        let mut w = UnicodeWidthStr::width(grapheme);

        // In CJK context, ambiguous width characters are width 2
        if cjk_context && is_east_asian_ambiguous(grapheme) {
            w = 2.max(w);
        }

        // Handle emojis and special characters
        if grapheme.chars().any(|c| {
            // Common emojis and symbols
            (c >= '\u{1F300}' && c <= '\u{1F9FF}') || // Emoticons, symbols
            (c >= '\u{2600}' && c <= '\u{26FF}') ||   // Misc symbols
            (c >= '\u{2700}' && c <= '\u{27BF}')      // Dingbats
        }) {
            w = 2; // Emojis are typically width 2
        }

        width += w;
    }
    width
}

/// Truncate string to fit within max columns, preserving grapheme clusters
pub fn visible_slice(s: &str, max_cols: usize, cjk: bool) -> (String, usize) {
    let mut used = 0;
    let mut result = String::new();

    for grapheme in s.graphemes(true) {
        let w = if cjk && is_east_asian_ambiguous(grapheme) {
            2
        } else {
            UnicodeWidthStr::width(grapheme)
        };

        if used + w > max_cols {
            // Check if we can fit an ellipsis
            if max_cols > used && max_cols - used >= 1 {
                result.push('…');
                used += 1;
            }
            break;
        }

        result.push_str(grapheme);
        used += w;
    }

    (result, used)
}

/// Pad string to exact width, considering CJK display width
pub fn pad_to_width(s: &str, target_width: usize, cjk: bool, align: Alignment) -> String {
    let current_width = display_width(s, cjk);

    if current_width >= target_width {
        // Truncate if too long
        let (truncated, _) = visible_slice(s, target_width, cjk);
        return truncated;
    }

    let padding = target_width - current_width;
    let spaces = " ".repeat(padding);

    match align {
        Alignment::Left => format!("{}{}", s, spaces),
        Alignment::Right => format!("{}{}", spaces, s),
        Alignment::Center => {
            let left_pad = padding / 2;
            let right_pad = padding - left_pad;
            format!("{}{}{}", " ".repeat(left_pad), s, " ".repeat(right_pad))
        }
    }
}

/// Text alignment options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alignment {
    Left,
    Right,
    Center,
}

/// Format commit message with proper CJK handling
pub fn format_commit_message(
    sha: &str,
    message: &str,
    max_width: usize,
    cjk: bool,
) -> String {
    // Reserve space for SHA (8 chars) + space
    let sha_width = 9;
    if max_width <= sha_width {
        return sha[..max_width.min(sha.len())].to_string();
    }

    let message_width = max_width - sha_width;
    let (truncated_msg, _) = visible_slice(message, message_width, cjk);

    format!("{} {}", &sha[..8.min(sha.len())], truncated_msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cjk_width() {
        // ASCII text
        assert_eq!(display_width("hello", false), 5);
        assert_eq!(display_width("hello", true), 5);

        // Chinese text
        assert_eq!(display_width("你好", false), 4);
        assert_eq!(display_width("你好", true), 4);

        // Mixed text
        assert_eq!(display_width("Hi你好", false), 6);
        assert_eq!(display_width("Hi你好", true), 6);

        // Emoji
        assert_eq!(display_width("😀", true), 2);
    }

    #[test]
    fn test_truncation() {
        let (result, width) = visible_slice("Hello世界", 7, true);
        assert_eq!(result, "Hello世");
        assert_eq!(width, 7);

        let (result, width) = visible_slice("Hello世界", 6, true);
        assert_eq!(result, "Hello…");
        assert_eq!(width, 6);
    }

    #[test]
    fn test_padding() {
        let padded = pad_to_width("Hi", 5, false, Alignment::Left);
        assert_eq!(padded, "Hi   ");

        let padded = pad_to_width("Hi", 5, false, Alignment::Right);
        assert_eq!(padded, "   Hi");

        let padded = pad_to_width("Hi", 5, false, Alignment::Center);
        assert_eq!(padded, " Hi  ");
    }
}