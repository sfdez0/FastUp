use crate::utils::{ensure_dir, get_log_file, get_logs_dir, strip_ansi_codes};
use chrono::Local;
use colored::*;
use core::fmt;
use std::fs::OpenOptions;
use std::io::Write;

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
    /// Function to get the string representation of the log level for file logging
    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Success => "SUCCESS",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

/// Function to print a log message with a timestamp and log level
pub fn print_log(level: LogLevel, msg: fmt::Arguments) {
    let timestamp = Local::now().format("%H:%M:%S");
    println!("[{}] FastUp {}: {}", timestamp, level.prefix(), msg);

    // Also write the log to the log file (without ANSI codes)
    let log_file = get_log_file();
    let logs_dir = get_logs_dir();
    let msg_string = format!("{}", msg);
    let clean_msg = strip_ansi_codes(&msg_string);

    if let Err(e) = ensure_dir(&logs_dir).and_then(|_| {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .and_then(|mut file| {
                writeln!(
                    file,
                    "[{}] FastUp {}: {}",
                    timestamp,
                    level.as_str(),
                    clean_msg
                )
            })
    }) {
        eprintln!("Failed to write log to file: {}", e);
    }
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
