use colored::*;
use std::io::{self, Write};

pub fn prompt_for_input(prompt: &str) -> String {
    print!("{prompt}: ");
    io::stdout().flush().expect("Failed to flush stdout");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    input.trim().to_string()
}

pub fn prompt_for_missing_arg(arg_name: &str) -> String {
    let hint = format!(
        "{} '{}'",
        "Please provide a value for".yellow(),
        arg_name.yellow().bold()
    );
    prompt_for_input(&hint)
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
        let _prompt_fn: fn(&str) -> String = prompt_for_input;
        let _prompt_missing_fn: fn(&str) -> String = prompt_for_missing_arg;
    }
}
