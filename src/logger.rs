use chrono::Local;
use core::fmt;

use colored::*;

/// Enum to represent different log levels
pub enum LogLevel {
    Info,
    Success,
    Warn,
    Error,
}

/// Implementation of LogLevel
impl LogLevel {
    /// Function to get the prefix
    fn prefix(&self) -> colored::ColoredString {
        match self {
            LogLevel::Info => "[INFO]".blue().bold(),
            LogLevel::Success => "[SUCCESS]".green().bold(),
            LogLevel::Warn => "[WARN]".yellow().bold(),
            LogLevel::Error => "[ERROR]".red().bold(),
        }
    }
}

/// Function to print a log message with a timestamp and log level
pub fn print_log(level: LogLevel, msg: fmt::Arguments) {
    let timestamp = Local::now().format("%H:%M:%S");
    println!("[{}] FastUp {}: {}", timestamp, level.prefix(), msg);
}

/// Prints an info message
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ($crate::logger::print_log($crate::logger::LogLevel::Info, format_args!($($arg)*)));
}

/// Prints a success message
#[macro_export]
macro_rules! success {
    ($($arg:tt)*) => ($crate::logger::print_log($crate::logger::LogLevel::Success, format_args!($($arg)*)));
}

/// Prints a warning message
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ($crate::logger::print_log($crate::logger::LogLevel::Warn, format_args!($($arg)*)));
}

/// Prints an error message
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ($crate::logger::print_log($crate::logger::LogLevel::Error, format_args!($($arg)*)));
}
