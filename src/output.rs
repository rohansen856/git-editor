//! Output routing.
//!
//! Human-readable progress goes through [`say!`](crate::say) /
//! [`say_inline!`](crate::say_inline): stdout normally, stderr in `--json`
//! mode, so stdout carries nothing but the JSON document.

use std::sync::atomic::{AtomicBool, Ordering};

static JSON_MODE: AtomicBool = AtomicBool::new(false);

pub fn set_json_mode(enabled: bool) {
    JSON_MODE.store(enabled, Ordering::Relaxed);
}

pub fn json_mode() -> bool {
    JSON_MODE.load(Ordering::Relaxed)
}

/// `println!` that moves to stderr in `--json` mode.
#[macro_export]
macro_rules! say {
    ($($arg:tt)*) => {
        if $crate::output::json_mode() {
            eprintln!($($arg)*)
        } else {
            println!($($arg)*)
        }
    };
}

/// `print!` that moves to stderr in `--json` mode.
#[macro_export]
macro_rules! say_inline {
    ($($arg:tt)*) => {
        if $crate::output::json_mode() {
            eprint!($($arg)*)
        } else {
            print!($($arg)*)
        }
    };
}
