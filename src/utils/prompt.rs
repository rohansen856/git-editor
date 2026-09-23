use crate::utils::types::Result;
use colored::*;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::io::{self, IsTerminal, Write};

/// Error message used for a user cancellation (Esc, Ctrl+C, declining a confirmation).
pub const CANCELLED: &str = "CANCELLED";

/// Exit status for a cancelled operation (same convention as SIGINT).
pub const EXIT_CANCELLED: i32 = 130;

pub fn is_cancelled(err: &dyn std::error::Error) -> bool {
    err.to_string() == CANCELLED
}

/// Process exit code for a top-level error (130 = cancelled, 1 = failure).
pub fn exit_code_for_error(err: &dyn std::error::Error) -> i32 {
    if is_cancelled(err) {
        EXIT_CANCELLED
    } else {
        1
    }
}

/// Report a cancellation and return the error that makes the process exit 130.
pub fn cancelled() -> Box<dyn std::error::Error> {
    cancelled_msg();
    CANCELLED.into()
}

fn print_esc_hint() {
    crate::say_inline!(" {}", "(Esc to cancel)".bright_black());
}

/// Read one line with live echo. Esc (or Ctrl+C) returns `Ok(None)`.
///
/// When stdin is not a terminal (pipes, CI, agents) a plain line is read
/// instead; end of input is an error rather than a hang.
pub fn read_line_allow_esc() -> Result<Option<String>> {
    if !io::stdin().is_terminal() {
        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 {
            crate::say!();
            return Err(
                "No input available (stdin is not a terminal and is closed); pass the values as flags and --yes to confirm"
                    .into(),
            );
        }
        crate::say!("{}", line.trim_end());
        return Ok(Some(line.trim_end_matches(['\r', '\n']).to_string()));
    }

    enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {e}"))?;

    let result = (|| -> Result<Option<String>> {
        let mut buffer = String::new();
        loop {
            let Event::Key(key) = event::read().map_err(|e| format!("Failed to read key: {e}"))?
            else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Esc => {
                    crate::say_inline!("\r\n");
                    let _ = io::stdout().flush();
                    return Ok(None);
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    crate::say_inline!("\r\n");
                    let _ = io::stdout().flush();
                    return Ok(None);
                }
                KeyCode::Enter => {
                    crate::say_inline!("\r\n");
                    let _ = io::stdout().flush();
                    return Ok(Some(buffer));
                }
                KeyCode::Backspace => {
                    if buffer.pop().is_some() {
                        crate::say_inline!("\x08 \x08");
                        let _ = io::stdout().flush();
                    }
                }
                KeyCode::Char(c) => {
                    buffer.push(c);
                    crate::say_inline!("{c}");
                    let _ = io::stdout().flush();
                }
                _ => {}
            }
        }
    })();

    let _ = disable_raw_mode();
    result
}

fn cancelled_msg() {
    crate::say!("{}", "Operation cancelled.".yellow());
}

pub fn prompt_for_input(prompt: &str) -> Result<String> {
    crate::say_inline!("{prompt}");
    print_esc_hint();
    crate::say_inline!(": ");
    io::stdout()
        .flush()
        .map_err(|e| format!("Failed to flush stdout: {e}"))?;

    match read_line_allow_esc()? {
        Some(input) => Ok(input.trim().to_string()),
        None => {
            cancelled_msg();
            Err(CANCELLED.into())
        }
    }
}

pub fn prompt_for_missing_arg(arg_name: &str) -> Result<String> {
    let hint = format!(
        "{} '{}'",
        "Please provide a value for".yellow(),
        arg_name.yellow().bold()
    );
    prompt_for_input(&hint)
}

/// Prompts with a suggested default (dimmed). Enter keeps default; Esc cancels.
pub fn prompt_with_default(prompt: &str, default_value: &str) -> Result<String> {
    crate::say_inline!(
        "{}: {} ",
        prompt.yellow().bold(),
        format!("({default_value})").bright_black()
    );
    print_esc_hint();
    crate::say_inline!(" ");
    io::stdout()
        .flush()
        .map_err(|e| format!("Failed to flush stdout: {e}"))?;

    match read_line_allow_esc()? {
        Some(input) => {
            let input = input.trim();
            if input.is_empty() {
                Ok(default_value.to_string())
            } else {
                Ok(input.to_string())
            }
        }
        None => {
            cancelled_msg();
            Err(CANCELLED.into())
        }
    }
}

/// Ask a yes/no question; `y`/`yes` (any case) confirms. With `assume_yes`
/// the question is answered automatically (for `--yes`).
pub fn confirm(question: &str, assume_yes: bool) -> Result<bool> {
    if assume_yes {
        crate::say!("{question} {}", "yes (--yes)".green());
        return Ok(true);
    }
    let answer = prompt_for_input(&format!("{question} (yes/no)"))?;
    Ok(is_yes(&answer))
}

fn is_yes(answer: &str) -> bool {
    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

/// Like [`read_prompted_line`] but keeps leading/trailing whitespace.
pub fn read_prompted_line_raw() -> Result<String> {
    match read_line_allow_esc()? {
        Some(input) => Ok(input),
        None => {
            cancelled_msg();
            Err(CANCELLED.into())
        }
    }
}

/// Prompt shown already printed by caller; reads a line or cancels.
pub fn read_prompted_line() -> Result<String> {
    match read_line_allow_esc()? {
        Some(input) => Ok(input.trim().to_string()),
        None => {
            cancelled_msg();
            Err(CANCELLED.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_yes_accepts_y_and_yes_only() {
        for yes in ["y", "Y", "yes", "YES", " yes "] {
            assert!(is_yes(yes), "{yes}");
        }
        for no in ["", "n", "no", "yep", "yess"] {
            assert!(!is_yes(no), "{no}");
        }
    }

    #[test]
    fn test_confirm_with_assume_yes_does_not_prompt() {
        assert!(confirm("Proceed?", true).unwrap());
    }

    #[test]
    fn test_is_cancelled() {
        let err: Box<dyn std::error::Error> = CANCELLED.into();
        assert!(is_cancelled(err.as_ref()));
        let other: Box<dyn std::error::Error> = "boom".into();
        assert!(!is_cancelled(other.as_ref()));
    }

    #[test]
    fn test_exit_code_for_cancelled_vs_error() {
        let cancelled: Box<dyn std::error::Error> = CANCELLED.into();
        assert_eq!(exit_code_for_error(cancelled.as_ref()), 130);

        let failed: Box<dyn std::error::Error> = "Invalid number".into();
        assert_eq!(exit_code_for_error(failed.as_ref()), 1);
    }

    #[test]
    fn test_cancelled_constant_stable() {
        // main.rs and prompt helpers must agree on this sentinel
        assert_eq!(CANCELLED, "CANCELLED");
        let err: Box<dyn std::error::Error> = CANCELLED.into();
        assert_eq!(exit_code_for_error(err.as_ref()), 130);
    }
}
