//! Neutralise terminal control sequences in repository-provided text.
//!
//! Author names and commit messages come from the repository (possibly a
//! cloned, untrusted one). Printing them raw would let embedded escape
//! sequences retitle, recolour or clear the user's terminal.

use std::borrow::Cow;

/// Replace control characters (other than tab and newline) with visible escapes.
pub fn safe(text: &str) -> Cow<'_, str> {
    if !text.chars().any(is_unsafe) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        if is_unsafe(c) {
            out.extend(c.escape_default());
        } else {
            out.push(c);
        }
    }
    Cow::Owned(out)
}

fn is_unsafe(c: char) -> bool {
    c.is_control() && c != '\n' && c != '\t'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_terminal_sequences() {
        assert_eq!(safe("a\u{1b}]0;title\u{7}b"), "a\\u{1b}]0;title\\u{7}b");
        assert_eq!(safe("red\u{1b}[31m"), "red\\u{1b}[31m");
        assert_eq!(safe("del\u{7f}"), "del\\u{7f}");
    }

    #[test]
    fn leaves_normal_text_alone() {
        assert!(matches!(safe("张三 <a@b.c>\tok\n"), Cow::Borrowed(_)));
    }
}
