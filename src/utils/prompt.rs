use std::io::{self, Write};

pub fn prompt_for_input(prompt: &str) -> String {
    print!("{}: ", prompt);
    io::stdout().flush().expect("Failed to flush stdout");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    input.trim().to_string()
}

pub fn prompt_for_missing_arg(arg_name: &str) -> String {
    let hint = format!("Please provide a value for '{}'", arg_name);
    prompt_for_input(&hint)
}
