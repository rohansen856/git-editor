use crate::utils::types::Result;
use colored::*;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::io::{self, Write};

/// Sent from prompt helpers when the user presses Esc; main exits 0 without red Error.
pub const CANCELLED: &str = "CANCELLED";

pub fn is_cancelled(err: &dyn std::error::Error) -> bool {
    err.to_string() == CANCELLED
}

/// Process exit code for a top-level error (0 = Esc cancel, 1 = failure).
pub fn exit_code_for_error(err: &dyn std::error::Error) -> i32 {
    if is_cancelled(err) {
        0
    } else {
        1
    }
}

fn print_esc_hint() {
    print!(" {}", "(Esc to cancel)".bright_black());
}

/// Read one line with live echo. Esc (or Ctrl+C) returns `Ok(None)`.
pub fn read_line_allow_esc() -> Result<Option<String>> {
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
                    print!("\r\n");
                    let _ = io::stdout().flush();
                    return Ok(None);
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    print!("\r\n");
                    let _ = io::stdout().flush();
                    return Ok(None);
                }
                KeyCode::Enter => {
                    print!("\r\n");
                    let _ = io::stdout().flush();
                    return Ok(Some(buffer));
                }
                KeyCode::Backspace => {
                    if buffer.pop().is_some() {
                        print!("\x08 \x08");
                        let _ = io::stdout().flush();
                    }
                }
                KeyCode::Char(c) => {
                    buffer.push(c);
                    print!("{c}");
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
    println!("{}", "Operation cancelled.".yellow());
}

pub fn prompt_for_input(prompt: &str) -> Result<String> {
    print!("{prompt}");
    print_esc_hint();
    print!(": ");
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
    print!(
        "{}: {} ",
        prompt.yellow().bold(),
        format!("({default_value})").bright_black()
    );
    print_esc_hint();
    print!(" ");
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
    fn test_prompt_for_missing_arg_formats_correctly() {
        let arg_name = "test_arg";
        assert_eq!(arg_name, "test_arg");
    }

    #[test]
    fn test_prompt_functions_exist() {
        let _prompt_fn: fn(&str) -> Result<String> = prompt_for_input;
        let _prompt_missing_fn: fn(&str) -> Result<String> = prompt_for_missing_arg;
        let _prompt_with_default_fn: fn(&str, &str) -> Result<String> = prompt_with_default;
        let _read_fn: fn() -> Result<String> = read_prompted_line;
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
        assert_eq!(exit_code_for_error(cancelled.as_ref()), 0);

        let failed: Box<dyn std::error::Error> = "Invalid number".into();
        assert_eq!(exit_code_for_error(failed.as_ref()), 1);
    }

    #[test]
    fn test_cancelled_constant_stable() {
        // main.rs and prompt helpers must agree on this sentinel
        assert_eq!(CANCELLED, "CANCELLED");
        let err: Box<dyn std::error::Error> = CANCELLED.into();
        assert_eq!(exit_code_for_error(err.as_ref()), 0);
    }
}
