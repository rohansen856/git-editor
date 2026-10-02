/// Rewrite author-related trailers in a commit message when identity changes.
///
/// Only the trailer block — the last paragraph of a message that has more
/// than one paragraph — is considered. `Signed-off-by`, `Co-authored-by` and
/// `Authored-by` lines there whose email matches `old_email`
/// (case-insensitive) are rewritten; everything else, including line endings
/// (`\n` or `\r\n`), is left byte-for-byte unchanged. Does not invent trailers.
pub fn rewrite_author_trailers(
    message: &str,
    old_name: &str,
    old_email: &str,
    new_name: &str,
    new_email: &str,
) -> String {
    if old_email == new_email && old_name == new_name {
        return message.to_string();
    }

    let lines: Vec<&str> = message.split_inclusive('\n').collect();
    let Some(block_start) = trailer_block_start(&lines) else {
        return message.to_string();
    };

    let mut out = String::with_capacity(message.len());
    for (i, line) in lines.iter().enumerate() {
        let (content, ending) = split_line_ending(line);
        match (i >= block_start)
            .then(|| rewrite_trailer_line(content, old_email, new_name, new_email))
            .flatten()
        {
            Some(rewritten) => {
                out.push_str(&rewritten);
                out.push_str(ending);
            }
            None => out.push_str(line),
        }
    }
    out
}

fn split_line_ending(line: &str) -> (&str, &str) {
    if let Some(content) = line.strip_suffix("\r\n") {
        (content, "\r\n")
    } else if let Some(content) = line.strip_suffix('\n') {
        (content, "\n")
    } else {
        (line, "")
    }
}

/// Index of the first line of the trailer block (last paragraph), if the
/// message has a paragraph before it.
fn trailer_block_start(lines: &[&str]) -> Option<usize> {
    let blank = |l: &&str| split_line_ending(l).0.trim().is_empty();
    let last_content = lines.iter().rposition(|l| !blank(l))?;
    let separator = lines[..last_content].iter().rposition(blank)?;
    // There must be a non-blank paragraph (e.g. the subject) before the block.
    lines[..separator]
        .iter()
        .any(|l| !blank(l))
        .then_some(separator + 1)
}

fn rewrite_trailer_line(
    line: &str,
    old_email: &str,
    new_name: &str,
    new_email: &str,
) -> Option<String> {
    let trimmed = line.trim_end();
    let (prefix, rest) = split_trailer(trimmed)?;
    if !is_author_trailer(prefix) {
        return None;
    }
    if !trailer_email_matches(rest, old_email) {
        return None;
    }
    Some(format!("{prefix}: {new_name} <{new_email}>"))
}

fn split_trailer(line: &str) -> Option<(&str, &str)> {
    let colon = line.find(':')?;
    let prefix = &line[..colon];
    let rest = line[colon + 1..].trim();
    if prefix.is_empty() {
        return None;
    }
    Some((prefix, rest))
}

fn is_author_trailer(prefix: &str) -> bool {
    matches!(
        prefix.to_ascii_lowercase().as_str(),
        "signed-off-by" | "co-authored-by" | "authored-by"
    )
}

fn trailer_email_matches(rest: &str, old_email: &str) -> bool {
    if let Some(email) = extract_angle_email(rest) {
        return email.eq_ignore_ascii_case(old_email);
    }
    // Bare email as the whole value, or trailing token
    let candidate = rest.trim();
    if candidate.eq_ignore_ascii_case(old_email) {
        return true;
    }
    candidate
        .split_whitespace()
        .next_back()
        .is_some_and(|tok| tok.eq_ignore_ascii_case(old_email))
}

fn extract_angle_email(rest: &str) -> Option<&str> {
    let start = rest.rfind('<')?;
    let end = rest[start + 1..].find('>')? + start + 1;
    Some(&rest[start + 1..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_signed_off_by_when_email_matches() {
        let msg = "Fix bug\n\nSigned-off-by: Old Name <old@ex.com>\n";
        let out = rewrite_author_trailers(msg, "Old Name", "old@ex.com", "New Name", "new@ex.com");
        assert_eq!(out, "Fix bug\n\nSigned-off-by: New Name <new@ex.com>\n");
    }

    #[test]
    fn leaves_other_signoffs_alone() {
        let msg = "Body\n\nSigned-off-by: Other <other@ex.com>\nSigned-off-by: Me <me@ex.com>\n";
        let out = rewrite_author_trailers(msg, "Me", "me@ex.com", "New", "new@ex.com");
        assert!(out.contains("Signed-off-by: Other <other@ex.com>"));
        assert!(out.contains("Signed-off-by: New <new@ex.com>"));
        assert!(!out.contains("me@ex.com"));
    }

    #[test]
    fn leaves_body_text_alone() {
        let msg = "Mention old@ex.com in body\n\nSigned-off-by: Old <old@ex.com>\n";
        let out = rewrite_author_trailers(msg, "Old", "old@ex.com", "New", "new@ex.com");
        assert!(out.starts_with("Mention old@ex.com in body\n"));
        assert!(out.contains("Signed-off-by: New <new@ex.com>"));
    }

    #[test]
    fn handles_bare_email_and_case() {
        let msg = "x\n\nsigned-off-by: Someone OLD@EX.COM\n";
        let out = rewrite_author_trailers(msg, "Someone", "old@ex.com", "New", "new@ex.com");
        assert!(out.contains("signed-off-by: New <new@ex.com>"));
    }

    #[test]
    fn no_change_when_email_differs() {
        let msg = "x\n\nSigned-off-by: Other <other@ex.com>\n";
        let out = rewrite_author_trailers(msg, "Me", "me@ex.com", "New", "new@ex.com");
        assert_eq!(out, msg);
    }

    #[test]
    fn rewrites_co_authored_and_authored_by() {
        let msg = "x\n\nCo-authored-by: Old <old@ex.com>\nAuthored-by: Old <old@ex.com>\n";
        let out = rewrite_author_trailers(msg, "Old", "old@ex.com", "New", "new@ex.com");
        assert!(out.contains("Co-authored-by: New <new@ex.com>"));
        assert!(out.contains("Authored-by: New <new@ex.com>"));
    }

    #[test]
    fn ignores_signoff_lines_outside_the_trailer_block() {
        let msg =
            "x\n\nAs documented:\nSigned-off-by: old@ex.com\n\nSigned-off-by: Old <old@ex.com>\n";
        let out = rewrite_author_trailers(msg, "Old", "old@ex.com", "New", "new@ex.com");
        assert_eq!(
            out,
            "x\n\nAs documented:\nSigned-off-by: old@ex.com\n\nSigned-off-by: New <new@ex.com>\n"
        );
    }

    #[test]
    fn subject_only_message_has_no_trailers() {
        let msg = "Signed-off-by: Old <old@ex.com>\n";
        assert_eq!(
            rewrite_author_trailers(msg, "Old", "old@ex.com", "New", "new@ex.com"),
            msg
        );
    }

    #[test]
    fn preserves_crlf_line_endings() {
        let msg = "subj\r\n\r\nbody\r\n\r\nSigned-off-by: Old <old@ex.com>\r\n";
        let out = rewrite_author_trailers(msg, "Old", "old@ex.com", "New", "new@ex.com");
        assert_eq!(
            out,
            "subj\r\n\r\nbody\r\n\r\nSigned-off-by: New <new@ex.com>\r\n"
        );
    }

    #[test]
    fn email_case_change_is_rewritten() {
        let msg = "x\n\nSigned-off-by: Me <me@ex.com>\n";
        let out = rewrite_author_trailers(msg, "Me", "me@ex.com", "Me", "Me@Ex.com");
        assert_eq!(out, "x\n\nSigned-off-by: Me <Me@Ex.com>\n");
    }

    #[test]
    fn noop_when_identity_unchanged() {
        let msg = "x\n\nSigned-off-by: Same <same@ex.com>\n";
        let out = rewrite_author_trailers(msg, "Same", "same@ex.com", "Same", "same@ex.com");
        assert_eq!(out, msg);
    }
}
