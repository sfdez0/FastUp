use clap::{Parser, Subcommand};
use colored::*;
use sysinfo::{Pid, System};

use fastup::config::refresh_status;
use fastup::elements::ElementType;
use fastup::state::FastUpState;
use fastup::utils::{check_port, get_process_listening_on_port, print_status};
use fastup::{error, info, success, warn};

/// Struct to define the CLI structure using clap
#[derive(Parser)]
#[command(
    name = "fastup",
    version = "0.1",
    author = "Sergio Fernández Verdugo",
    about = "A lightweight YAML-based local process manager to automate your development environment"
)]
struct Cli {
    #[command(subcommand)]
    fastup_command: Commands,
}

/// Enum to define subcommands for the CLI.
/// - `up`: Starts an element
/// - `status`: Checks the status of the elements.
/// - `down`: Closes an element
#[derive(Subcommand)]
enum Commands {
    /// Start an element
    Up {
        /// Name of the element to start
        name: String,
    },
    /// Close the element
    Down {
        /// Name of the element to stop
        name: String,
    },
    /// Check the status of the elements
    Status,
}

/// Main function that parses the CLI arguments and executes the corresponding command.
fn main() {
    let cli = Cli::parse();

    match &cli.fastup_command {
        Commands::Up { name } => {
            cmd_up(name);
        }
        Commands::Status => {
            cmd_status(true);
        }
        Commands::Down { name } => {
            cmd_down(name);
        }
    }
}

/// Function to start an element as defined in the config file
/// - `name`: Name of the element to start
fn cmd_up(name: &str) {
    // Refresh the status and load the config
    let config = refresh_status();

    // Find the element in the config by name
    let element = match config.elements_config.iter().find(|e| e.name == name) {
        Some(elem) => elem,
        None => {
            error!(
                "Element not found inside fastup.yaml: {}",
                name.red().bold()
            );
            return;
        }
    };

    info!("Starting element {}...", element.name.green().bold());

    match &element.element_type {
        ElementType::Command { .. } => {
            // Command -> Start the element, then save its PID in the state file
            match element.element_type.start() {
                Ok(pid) => {
                    let mut state = FastUpState::load();
                    if let Err(e) =
                        state.register_element(element.name.clone(), pid, "Command".to_string())
                    {
                        warn!("Failed to save command element state: {}", e);
                    }

                    info!(
                        "Command element {} started with PID: {}",
                        element.name.green().bold(),
                        pid.to_string().green()
                    );
                }
                Err(e) => error!(
                    "Failed to start command element {}: {}",
                    element.name.red().bold(),
                    e
                ),
            }
        }
        ElementType::Service { .. } => {
            // Service -> Start the element using systemctl, then save it in the state as a service element
            match element.element_type.start() {
                Ok(_) => {
                    let mut state = FastUpState::load();
                    if let Err(e) =
                        state.register_element(element.name.clone(), 0, "Service".to_string())
                    {
                        warn!("Warning: Failed to save service element state: {}", e);
                    }

                    success!(
                        "Service element {} started successfully",
                        element.name.green().bold()
                    );
                }
                Err(e) => error!(
                    "Failed to start service element {}: {}",
                    element.name.red().bold(),
                    e
                ),
            }
        }
    }
}

/// Function to check the status of the elements defined in the config file and print it in a formatted way
/// - `print`: Whether to print the status of each element
fn cmd_status(print: bool) {
    // Refresh the status and load the config
    let config = refresh_status();

    if print {
        info!("Checking element status...");
        println!("{:-<45}", "");
    }

    if print {
        // Print the status of each element defined in the config file
        for element in &config.elements_config {
            let online = check_port("127.0.0.1", element.port);
            print_status(&element.name, element.port, online);
        }
    }
}

/// Function to stop the element as defined in the config file
fn cmd_down(name: &str) {
    // Refresh the status and load the config
    let config = refresh_status();

    // Find the element in the config by name
    let element = match config.elements_config.iter().find(|e| e.name == name) {
        Some(elem) => elem,
        None => {
            error!(
                "Element not found inside fastup.yaml: {}",
                name.red().bold()
            );
            return;
        }
    };

    info!("Stopping element {}...", element.name.green().bold());

    let mut state = FastUpState::load();

    match &element.element_type {
        ElementType::Command { .. } => {
            // Command -> Try to stop the element using the cached PID
            if let Some(info) = state.get_element(&element.name) {
                let mut pid = info.pid;

                // Refresh system info (only processes)
                let mut sys = System::new();
                // Only need PIDs running, no need to refresh other info to minimize the time spent refreshing
                sys.refresh_processes_specifics(
                    sysinfo::ProcessesToUpdate::All,
                    true,
                    sysinfo::ProcessRefreshKind::nothing(),
                );

                // If the cached PID is not running, try to find the current PID of the element.
                if sys.process(Pid::from(pid)).is_none() {
                    warn!(
                        "Warning: Cached PID {} is no longer running. Searching for current process...",
                        pid.to_string().yellow().bold()
                    );

                    // Try to find the current PID of the element by checking the port
                    if let Some(new_pid) = get_process_listening_on_port(element.port) {
                        info!(
                            "Found element {} on port {} with PID: {}",
                            element.name.green().bold(),
                            element.port.to_string().green(),
                            new_pid.to_string().green()
                        );

                        pid = new_pid;
                    }

                    // If the PID is still not found or not running, print an error and return
                    if pid == 0 || sys.process(Pid::from(pid)).is_none() {
                        error!(
                            "Could not find a running process for command element {}. It may have already been stopped.",
                            element.name.red().bold()
                        );
                        return;
                    }
                }

                match element.element_type.stop(pid, &mut state, &element.name) {
                    Ok(_) => { /* Success message already printed in stop() */ }
                    Err(e) => error!(
                        "Failed to stop command element {}: {}",
                        element.name.red().bold(),
                        e
                    ),
                }
            } else {
                error!(
                    "No record found for command element {}. It might not have been started with fastup or it was already closed.",
                    element.name.red().bold()
                );
            }
        }
        ElementType::Service { .. } => {
            // Service -> Stop the service using systemctl
            match element.element_type.stop(0, &mut state, &element.name) {
                Ok(_) => { /* Success message already printed in stop() */ }
                Err(e) => error!(
                    "Failed to stop service element {}: {}",
                    element.name.red().bold(),
                    e
                ),
            }
        }
    }
}
