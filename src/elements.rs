use colored::*;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::process::{Command, Stdio};
use sysinfo::{Pid, System};

use crate::state::FastUpState;

/// Struct to store information about a running element
#[derive(Serialize, Deserialize, Clone)]
pub struct ElementInfo {
    /// PID executing the element
    pub pid: usize,
    /// Name of the element
    pub name: String,
    /// Start time of the element
    pub start_time: u64,
    /// Type of element: "Command" or "System"
    pub element_type: String,
    /// Whether the element was started by fastup or is externally running
    pub started_by_fastup: bool,
}

/// Struct to represent the configuration of an element as defined in the YAML config file
#[derive(Serialize, Deserialize)]
pub struct ElementConfig {
    /// Name of the element
    pub name: String,
    /// Port on which the element is expected to run
    pub port: u16,
    /// Element type (Command or System)
    #[serde(flatten)]
    pub element_type: ElementType,
}

/// Enum to represent the type of element and how to start/stop it
#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "element_type")]
pub enum ElementType {
    /// Element that is started with a custom command
    Command {
        /// Command to start the element
        start_command: String,
        /// Arguments for the start command
        args: Vec<String>,
        /// Optional PID route to validate the element status
        log_file: Option<String>,
    },
    /// Service that is started as a system service
    Service {
        /// Name of the service
        service_name: String,
    },
}

/// Implementation of the ElementType enum
impl ElementType {
    /// Start the element and return its PID
    pub fn start(&self) -> std::io::Result<u32> {
        match self {
            ElementType::Command {
                start_command,
                args,
                log_file,
            } => {
                // Command -> We start the element with the provided command and arguments
                let mut process = Command::new(start_command);
                process.args(args);

                // If a log file is provided, redirect stdout and stderr to that file. Otherwise, discard the output
                if let Some(path) = log_file {
                    let file = OpenOptions::new().create(true).append(true).open(path)?;

                    process.stdout(Stdio::from(file.try_clone()?));
                    process.stderr(Stdio::from(file));
                } else {
                    process.stdout(Stdio::null());
                    process.stderr(Stdio::null());
                }

                // Spawn the process and return its PID
                let child = process.spawn()?;
                Ok(child.id())
            }
            ElementType::Service { service_name } => {
                // Service -> We start the service using systemctl
                Command::new("sudo")
                    .arg("systemctl")
                    .arg("start")
                    .arg(service_name)
                    .stdin(Stdio::inherit()) // Inherit stdin to allow password input for sudo if needed
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()?
                    .wait()?; // Wait for the command to finish before returning

                // Return 0, PID is not tracked for system services
                Ok(0)
            }
        }
    }

    /// Stop the element given its PID and element type
    pub fn stop(&self, pid: usize, state: &mut FastUpState, command: &str) -> std::io::Result<()> {
        match self {
            ElementType::Command {
                start_command,
                args: _,
                log_file: _,
            } => {
                // Command -> Kill the process with the given PID, first validating that it is the expected element
                if state.validate(command, start_command) {
                    let sys = System::new_all();
                    if let Some(process) = sys.process(Pid::from(pid)) {
                        process.kill();

                        // Remove the element from the state and save
                        state.elements.remove(command);
                        state.save()?;

                        println!("Element {} stopped with PID: {}", command.green(), pid);
                    }
                } else {
                    println!(
                        "The PID {} is not running the expected element {}. It might have been started manually or by another tool.",
                        pid,
                        command.red()
                    );
                }
                Ok(())
            }
            ElementType::Service { service_name } => {
                // Service -> Stop the service using systemctl
                let mut child = Command::new("sudo")
                    .arg("systemctl")
                    .arg("stop")
                    .arg(service_name)
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()?;

                // Wait for the command to finish, then remove the service from the state and save
                child.wait()?;
                state.elements.remove(command);
                state.save()?;

                println!("Service {} stopped", service_name.green());
                Ok(())
            }
        }
    }
}
