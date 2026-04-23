use colored::*;
use std::env;
use std::fs;
use std::net::{TcpStream, ToSocketAddrs};
use std::process::Command;
use std::time::Duration;

use crate::error;

/// Function to get the home directory of the current user
fn get_home_dir() -> String {
    env::var("HOME").unwrap_or_else(|_| ".".to_string())
}

/// Function to get the configuration directory path
pub fn get_config_dir() -> String {
    format!("{}/.config/fastup", get_home_dir())
}

/// Function to get the data directory path
pub fn get_data_dir() -> String {
    format!("{}/.local/share/fastup", get_home_dir())
}

/// Function to get the configuration file path
pub fn get_config_file() -> String {
    format!("{}/fastup.yaml", get_config_dir())
}

/// Function to get the logs directory path
pub fn get_logs_dir() -> String {
    format!("{}/logs", get_data_dir())
}

/// Function to get the log file path
pub fn get_log_file() -> String {
    format!("{}/fastup.txt", get_logs_dir())
}

/// Function to get the state file path
pub fn get_state_file() -> String {
    format!("{}/state.json", get_data_dir())
}

/// Function to check if a specific port is open on the localhost
/// - `host`: Host to check
/// - `port`: Port to check
pub fn check_port(host: &str, port: u16) -> bool {
    let direction = format!("{}:{}", host, port);

    // Try to connect to the port with a short timeout. If it succeeds, the element is online.
    TcpStream::connect_timeout(
        &direction.to_socket_addrs().unwrap().next().unwrap(),
        Duration::from_millis(250),
    )
    .is_ok()
}

/// Function to try to find the PID of a process listening on a specific port
/// - `port`: Port to check
pub fn get_process_listening_on_port(port: u16) -> Option<usize> {
    let output = std::process::Command::new("ss").arg("-tlnp").output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Parse the output to find the line corresponding to the port and extract the PID
        for line in stdout.lines() {
            // Look for lines that contain the port and are in LISTEN state
            if line.contains(&format!(":{}", port)) && line.contains("LISTEN") {
                // Format: "users:(("process_name",pid=XXXX,fd=YY))"
                // Extract everything after "pid=" until the next comma or closing paren
                if let Some(pid_start) = line.find("pid=") {
                    let pid_str = &line[pid_start + 4..];
                    if let Some(pid_end) = pid_str.find(|c: char| !c.is_numeric())
                        && let Ok(pid) = pid_str[..pid_end].parse::<usize>()
                    {
                        return Some(pid);
                    }
                }
            }
        }
    }

    None
}

/// Function to check if a system element is active using systemctl
/// - `element_name`: Name of the binary to check
pub fn is_service_active(element_name: &str) -> bool {
    match Command::new("systemctl")
        .arg("is-active")
        .arg("--quiet")
        .arg(element_name)
        .status()
    {
        Ok(status) => status.success(),
        Err(e) => {
            error!("Error occurred while checking element status: {}", e);
            false
        }
    }
}

/// Function to print the status of a element in a formatted way
/// - `name`: Name of the element
/// - `port`: Port of the element
/// - `online`: Whether the element is online or offline
pub fn print_status(name: &str, port: u16, online: bool) {
    // Format the status text with colors
    let status_text = if online {
        "ONLINE".on_green().white().bold()
    } else {
        "OFFLINE".on_red().white().bold()
    };

    // Print the element name, port, and status
    println!("{:<20} | Port: {:<5}| {}", name.blue(), port, status_text);
}

/// Function to ensure that a directory exists, creating it if necessary
pub fn ensure_dir(dir: &str) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    Ok(())
}

/// Function to remove ANSI escape codes from a string
pub fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;

    for ch in s.chars() {
        if ch == '\x1b' {
            in_escape = true;
        } else if in_escape && ch == 'm' {
            in_escape = false;
        } else if !in_escape {
            result.push(ch);
        }
    }

    result
}
