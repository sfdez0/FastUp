use colored::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self};
use std::path::Path;
use sysinfo::{Pid, System};

use crate::elements::ElementInfo;

/// Path to the state file
pub const STATE_FILE: &str = "logs/fastup_state.json";

/// Struct to represent the state of the application
#[derive(Default, Serialize, Deserialize)]
pub struct FastUpState {
    /// Map of element names to their information
    pub elements: HashMap<String, ElementInfo>,
}

/// Implementation of the FastUpState struct.
impl FastUpState {
    /// Function to load the status from the state file
    pub fn load() -> Self {
        // Read the state file and deserialize it, or return an empty state if the file doesn't exist or is invalid
        fs::read_to_string(STATE_FILE)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    /// Function to save the current state to the state file
    /// - `self`: The current state to be saved
    pub fn save(&self) -> std::io::Result<()> {
        // Ensure the directory exists
        if let Some(parent) = Path::new(STATE_FILE).parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }

        // Serialize the state to JSON and write it to the file
        let content = serde_json::to_string_pretty(self).unwrap();
        fs::write(STATE_FILE, content)
    }

    /// Register a newly started element in the state and save it
    /// - `name`: Name of the element
    /// - `pid`: PID of the process running the element
    /// - `element_type`: Type of the element (Command or System)
    pub fn register_element(
        &mut self,
        name: String,
        pid: u32,
        element_type: String,
    ) -> std::io::Result<()> {
        // Insert the element information into the state and save it
        self.elements.insert(
            name.clone(),
            ElementInfo {
                pid: pid as usize,
                name,
                start_time: 0,
                element_type,
                started_by_fastup: true, // Mark this element as started by fastup
            },
        );
        self.save()
    }

    /// Register an element that is already running but wasn't started by fastup
    /// - `name`: Name of the element
    /// - `pid`: PID of the process running the element (if known, otherwise 0)
    /// - `element_type`: Type of the element (Command or System)
    pub fn register_external_element(
        &mut self,
        name: String,
        pid: usize,
        element_type: String,
    ) -> std::io::Result<()> {
        // Insert the element information into the state and save it
        self.elements.insert(
            name.clone(),
            ElementInfo {
                pid,
                name,
                start_time: 0,
                element_type,
                started_by_fastup: false, // Mark this element as not started by fastup
            },
        );
        self.save()
    }

    /// Get the element info if it exists
    /// - `name`: Name of the element to retrieve
    pub fn get_element(&self, name: &str) -> Option<&ElementInfo> {
        self.elements.get(name)
    }

    /// Function to validate if the PID stored for an element is still running.
    /// If the PID is running but with a different element, it is removed from the state.
    /// - `command`: Name of the element to validate
    /// - `element_start_cmd`: Expected start command for the element, used to verify that the running PID corresponds to the correct element
    pub fn validate(&self, command: &str, element_start_cmd: &str) -> bool {
        println!("Validating element '{}'...", command);

        if let Some(info) = self.elements.get(command) {
            // Refresh system info to get the current processes
            let mut sys = System::new_all();
            sys.refresh_all();

            // Check if the PID is still running and corresponds to the expected element
            if let Some(process) = sys.process(Pid::from(info.pid)) {
                let full_cmd = process
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ");

                // Check if the full command contains the expected start command for the element
                let res = full_cmd.contains(element_start_cmd);
                if res {
                    println!(
                        "PID {} is running and matches the expected command for element '{}'.",
                        info.pid,
                        command.green()
                    );
                } else {
                    println!(
                        "PID {} is running but does not match the expected command for element '{}'.",
                        info.pid,
                        command.red()
                    );
                }
                return res;
            }
        }

        // Element not valid
        false
    }

    /// Check which elements are still running and remove dead ones from the state.
    /// Returns the updated list of running elements.
    /// - `self`: The current state to be cleaned up
    pub fn cleanup_dead_elements(&mut self) -> std::io::Result<()> {
        // Refresh system info to get the current processes
        let mut sys = System::new_all();
        sys.refresh_all();

        // Remove elements whose PIDs are no longer running
        self.elements
            .retain(|_, info| sys.process(Pid::from(info.pid)).is_some());

        // Save the updated state
        self.save()
    }
}
