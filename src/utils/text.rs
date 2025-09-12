use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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
