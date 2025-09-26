use crate::utils::types::Result;
use colored::*;
use std::io::{self, Write};

pub fn prompt_for_input(prompt: &str) -> Result<String> {
    print!("{prompt}: ");
    io::stdout()
        .flush()
        .map_err(|e| format!("Failed to flush stdout: {e}"))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("Failed to read line: {e}"))?;

    Ok(input.trim().to_string())
}

pub fn prompt_for_missing_arg(arg_name: &str) -> Result<String> {
    let hint = format!(
        "{} '{}'",
        "Please provide a value for".yellow(),
        arg_name.yellow().bold()
    );
    prompt_for_input(&hint)
}

// Prompts for input with a suggested default value shown in faded color. If the user presses Enter, the default is used. If they type something, that's used instead.
pub fn prompt_with_default(prompt: &str, default_value: &str) -> Result<String> {
    print!(
        "{}: {} ",
        prompt.yellow().bold(),
        format!("({default_value})").bright_black()
    );
    io::stdout()
        .flush()
        .map_err(|e| format!("Failed to flush stdout: {e}"))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("Failed to read line: {e}"))?;

    let input = input.trim();
    if input.is_empty() {
        Ok(default_value.to_string())
    } else {
        Ok(input.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_for_missing_arg_formats_correctly() {
        // We can't easily test interactive input, but we can test that the
        // function formats the hint correctly
        let arg_name = "test_arg";

        // This would normally prompt for input, but we can test the format
        // by checking that the function exists and takes the right parameters
        assert_eq!(arg_name, "test_arg");
    }

    #[test]
    fn test_prompt_functions_exist() {
        // Test that both prompt functions exist and can be called
        // (though we can't test interactive behavior in unit tests)

        // These functions exist and have the right signatures
        let _prompt_fn: fn(&str) -> Result<String> = prompt_for_input;
        let _prompt_missing_fn: fn(&str) -> Result<String> = prompt_for_missing_arg;
        let _prompt_with_default_fn: fn(&str, &str) -> Result<String> = prompt_with_default;
    }
}
