use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Format ISO datetime string to simple date format
///
/// # Examples
/// ```
/// use mbr_core::utils::text::format_datetime;
/// let datetime = "2023-12-25T10:30:00.000Z";
/// assert_eq!(format_datetime(datetime), "2023-12-25");
/// ```
pub fn format_datetime(datetime: &str) -> String {
    if let Some(date_part) = datetime.split('T').next() {
        date_part.to_string()
    } else {
        datetime.chars().take(16).collect()
    }
}

/// Wrap text to fit within specified width, breaking at word boundaries
///
/// # Examples
/// ```
/// use mbr_core::utils::text::wrap_text;
/// let text = "This is a long text that needs wrapping";
/// let wrapped = wrap_text(text, 10);
/// assert_eq!(wrapped[0], "This is a");
/// ```
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::with_capacity(max_width);

    for word in text.split_whitespace() {
        // If adding this word would exceed the width
        if current_line.len() + word.len() + 1 > max_width {
            if !current_line.is_empty() {
                lines.push(current_line);
                current_line = String::with_capacity(max_width);
            }

            // If a single word is longer than max_width, break it
            if word.len() > max_width {
                let mut remaining = word;
                while remaining.len() > max_width {
                    lines.push(remaining[..max_width].to_string());
                    remaining = &remaining[max_width..];
                }
                if !remaining.is_empty() {
                    current_line = remaining.to_string();
                }
            } else {
                current_line = word.to_string();
            }
        } else {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}

/// Truncate text with unicode support (main truncate function)
/// Alias for truncate_text_unicode for backward compatibility
pub fn truncate_text(text: &str, max_width: usize) -> String {
    truncate_text_unicode(text, max_width)
}

pub fn truncate_text_unicode(text: &str, max_width: usize) -> String {
    if text.width() <= max_width {
        return text.to_string();
    }

    const ELLIPSIS: &str = "...";
    let ellipsis_width = ELLIPSIS.width();

    if max_width <= ellipsis_width {
        return ELLIPSIS[..max_width].to_string();
    }

    let target_width = max_width - ellipsis_width;
    let mut result = String::new();
    let mut current_width = 0;

    for ch in text.chars() {
        let ch_width = ch.width().unwrap_or(0);
        if current_width + ch_width > target_width {
            break;
        }
        result.push(ch);
        current_width += ch_width;
    }

    result.push_str(ELLIPSIS);
    result
}

pub fn truncate_text_simple(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        text.to_string()
    } else if max_length <= 3 {
        "...".to_string()
    } else {
        format!("{}...", &text[..max_length - 3])
    }
}

pub fn pad_to_width(text: &str, width: usize) -> String {
    let text_width = text.width();
    if text_width >= width {
        text.to_string()
    } else {
        format!("{}{}", text, " ".repeat(width - text_width))
    }
}

pub fn center_text(text: &str, width: usize) -> String {
    let text_width = text.width();
    if text_width >= width {
        return text.to_string();
    }

    let padding = width - text_width;
    let left_padding = padding / 2;
    let right_padding = padding - left_padding;

    format!(
        "{}{}{}",
        " ".repeat(left_padding),
        text,
        " ".repeat(right_padding)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_datetime() {
        assert_eq!(format_datetime("2023-12-25T10:30:00.000Z"), "2023-12-25");
        assert_eq!(format_datetime("2023-12-25"), "2023-12-25");
        assert_eq!(format_datetime("invalid"), "invalid");
        assert_eq!(format_datetime(""), "");
    }

    #[test]
    fn test_wrap_text() {
        let text = "This is a long text that needs wrapping";
        let wrapped = wrap_text(text, 10);
        assert_eq!(wrapped.len(), 4);
        assert_eq!(wrapped[0], "This is a");
        assert_eq!(wrapped[1], "long text");
        assert_eq!(wrapped[2], "that needs");
        assert_eq!(wrapped[3], "wrapping");

        // Test empty text
        assert_eq!(wrap_text("", 10), vec![""]);

        // Test single word longer than max width
        let long_word = "superlongword";
        let wrapped = wrap_text(long_word, 5);
        assert_eq!(wrapped[0], "super");
        assert_eq!(wrapped[1], "longw");
        assert_eq!(wrapped[2], "ord");
    }

    #[test]
    fn test_truncate_text() {
        // Test alias function
        assert_eq!(truncate_text("Hello", 10), "Hello");
        assert_eq!(truncate_text("Hello World!", 8), "Hello...");
    }

    #[test]
    fn test_truncate_text_unicode() {
        assert_eq!(truncate_text_unicode("Hello", 10), "Hello");
        assert_eq!(truncate_text_unicode("Hello World!", 8), "Hello...");
        assert_eq!(truncate_text_unicode("", 5), "");
    }

    #[test]
    fn test_truncate_text_simple() {
        assert_eq!(truncate_text_simple("short", 10), "short");
        assert_eq!(
            truncate_text_simple("this is a long text", 10),
            "this is..."
        );
        assert_eq!(truncate_text_simple("test", 3), "...");
        assert_eq!(truncate_text_simple("", 5), "");
    }

    #[test]
    fn test_pad_to_width() {
        assert_eq!(pad_to_width("Hello", 10), "Hello     ");
        assert_eq!(pad_to_width("Hello World", 5), "Hello World");
        assert_eq!(pad_to_width("Text", 10), "Text      ");
    }

    #[test]
    fn test_center_text() {
        assert_eq!(center_text("Hi", 6), "  Hi  ");
        assert_eq!(center_text("Hello", 5), "Hello");
        assert_eq!(center_text("Hi", 12), "     Hi     ");
    }
}
