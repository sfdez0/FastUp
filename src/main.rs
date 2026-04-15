use clap::{Parser, Subcommand};
use colored::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use sysinfo::{Pid, System};

/// Path to the state file
const STATE_FILE: &str = "logs/fastup_state.json";
/// Path to the config file
const CONFIG_FILE: &str = "config/fastup.yaml";

/// Struct to store information about a running element
#[derive(Serialize, Deserialize, Clone)]
struct ElementInfo {
    /// PID executing the element
    pid: usize,
    /// Name of the element
    name: String,
    /// Start time of the element
    start_time: u64,
    /// Type of element: "Command" or "System"
    element_type: String,
    /// Whether the element was started by fastup or is externally running
    started_by_fastup: bool,
}

/// Struct to represent the configuration of an element as defined in the YAML config file
#[derive(Serialize, Deserialize)]
struct ElementConfig {
    /// Name of the element
    name: String,
    /// Port on which the element is expected to run
    port: u16,
    /// Element type (Command or System)
    #[serde(flatten)]
    element_type: ElementType,
}

/// Enum to represent the type of element and how to start/stop it
#[derive(Serialize, Deserialize)]
#[serde(tag = "element_type")]
enum ElementType {
    /// Element that is started with a custom command
    Command {
        /// Command to start the element
        command: String,
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

/// Struct to represent the state of the application
#[derive(Default, Serialize, Deserialize)]
struct FastUpState {
    /// Map of element names to their information
    elements: HashMap<String, ElementInfo>,
}

/// Struct to represent the configuration of the application as defined in the YAML config file
#[derive(Deserialize)]
struct FastUpConfig {
    /// List of elements defined in the config file
    elements_config: Vec<ElementConfig>,
}

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
/// - `close`: Closes an element
#[derive(Subcommand)]
enum Commands {
    /// Start an element
    Up {
        /// Name of the element to start
        name: String,
    },
    /// Close the element
    Close {
        /// Name of the element to close
        name: String,
    },
    /// Check the status of the elements
    Status,
}

/// Implementation of the ElementType enum
impl ElementType {
    /// Start the element and return its PID
    fn start(&self) -> std::io::Result<u32> {
        match self {
            ElementType::Command {
                command,
                args,
                log_file,
            } => {
                // Command -> We start the element with the provided command and arguments
                let mut process = Command::new(command);
                process.args(args);

                // If a log file is provided, redirect stdout and stderr to that file. Otherwise, discard the output.
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
    fn stop(&self, pid: usize, state: &mut FastUpState, command: &str) -> std::io::Result<()> {
        println!("Stopping element {}...", command.green());

        match self {
            ElementType::Command {
                command,
                args: _,
                log_file: _,
            } => {
                // Command -> Kill the process with the given PID, first validating that it is the expected element
                if state.validate(command, command) {
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

/// Implementation of the FastUpState struct.
impl FastUpState {
    /// Function to load the status from the state file
    fn load() -> Self {
        // Read the state file and deserialize it, or return an empty state if the file doesn't exist or is invalid
        fs::read_to_string(STATE_FILE)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    /// Function to save the current state to the state file
    /// - `self`: The current state to be saved
    fn save(&self) -> std::io::Result<()> {
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
    fn register_element(
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
    fn register_external_element(
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
    fn get_element(&self, name: &str) -> Option<&ElementInfo> {
        self.elements.get(name)
    }

    /// Function to validate if the PID stored for an element is still running.
    /// If the PID is running but with a different element, it is removed from the state.
    /// - `command`: Name of the element to validate
    /// - `element_start_cmd`: Expected start command for the element, used to verify that the running PID corresponds to the correct element
    fn validate(&self, command: &str, element_start_cmd: &str) -> bool {
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
    fn cleanup_dead_elements(&mut self) -> std::io::Result<()> {
        // Refresh system info to get the current processes
        let mut sys = System::new_all();
        sys.refresh_all();

        // Collect the names of elements whose PIDs are no longer running
        let dead_elements: Vec<String> = self
            .elements
            .iter()
            .filter_map(|(name, info)| {
                if sys.process(Pid::from(info.pid)).is_none() {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        // Remove dead elements from the state
        for command in dead_elements {
            self.elements.remove(&command);
        }

        // Save the updated state
        self.save()
    }
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
        Commands::Close { name } => {
            cmd_close(name);
        }
    }
}

/// Function to load the configuration from the YAML file
fn load_config() -> FastUpConfig {
    let content = fs::read_to_string(CONFIG_FILE).expect("Could not find the fastup.yaml file");

    serde_yml::from_str(&content).expect("Error parsing the fastup.yaml file")
}

/// Function to start an element as defined in the config file
/// - `name`: Name of the element to start
fn cmd_up(name: &str) {
    // Refresh the status and load the config
    let config = refresh_status();

    // Find the element in the config by name
    let element = config
        .elements_config
        .iter()
        .find(|e| e.name == name)
        .expect("Element not found inside fastup.yaml");

    println!("Starting element {}...", element.name.green());

    match &element.element_type {
        ElementType::Command { .. } => {
            // Command -> Start the element, then save its PID in the state file
            match element.element_type.start() {
                Ok(pid) => {
                    let mut state = FastUpState::load();
                    if let Err(e) =
                        state.register_element(element.name.clone(), pid, "Command".to_string())
                    {
                        eprintln!("Warning: Failed to save command element state: {}", e);
                    }
                    println!(
                        "Command element {} started with PID: {}",
                        element.name.green(),
                        pid
                    );
                }
                Err(e) => eprintln!(
                    "Failed to start command element {}: {}",
                    element.name.red(),
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
                        eprintln!("Warning: Failed to save service element state: {}", e);
                    }

                    println!(
                        "Service element {} started successfully",
                        element.name.green()
                    );
                }
                Err(e) => eprintln!(
                    "Failed to start service element {}: {}",
                    element.name.red(),
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
        println!("FastUp: Checking element status...");
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

/// Function to refresh the status of the elements by cleaning up dead elements from the state and checking for externally running elements
/// It also returns the loaded configuration.
fn refresh_status() -> FastUpConfig {
    // Load the config and the current state
    let config = load_config();
    let mut state = FastUpState::load();

    // Clean up dead elements from state and save
    if let Err(e) = state.cleanup_dead_elements() {
        eprintln!("Warning: Failed to update element state: {}", e);
    }

    // Check for externally running elements
    for element in &config.elements_config {
        let online = check_port("127.0.0.1", element.port);

        // If element is online but not in state, it's running externally
        if online && !state.elements.contains_key(&element.name) {
            let element_type_str = match &element.element_type {
                ElementType::Command { .. } => "Command",
                ElementType::Service { .. } => "Service",
            };

            let is_service_element = matches!(element.element_type, ElementType::Service { .. });
            let mut is_service_element_active = false;
            let mut pid = 0;
            if is_service_element {
                // The PID should not be tracked for service elements
                is_service_element_active = is_service_active(match &element.element_type {
                    ElementType::Service { service_name } => service_name,
                    _ => unreachable!(),
                });
            } else {
                // The PID should be tracked for direct elements
                pid = get_process_listening_on_port(element.port).unwrap_or(0);
            }

            // Register the element if a PID is found, or if it's an active service element
            if (pid != 0 || is_service_element_active)
                && let Err(e) = state.register_external_element(
                    element.name.clone(),
                    pid,
                    element_type_str.to_string(),
                )
            {
                eprintln!(
                    "Warning: Failed to register external element {}: {}",
                    element.name, e
                );
            }
        }
    }

    config
}

/// Function to close the element as defined in the config file
fn cmd_close(name: &str) {
    // Refresh the status and load the config
    let config = refresh_status();

    // Find the element in the config by name
    let element = config
        .elements_config
        .iter()
        .find(|e| e.name == name)
        .expect("ERROR: Element not found inside fastup.yaml");

    println!("Closing element {}...", element.name.green());

    let mut state = FastUpState::load();

    match &element.element_type {
        ElementType::Command { .. } => {
            // Command -> Try to stop the element using the cached PID
            if let Some(info) = state.get_element(&element.name).cloned() {
                let mut pid = info.pid;

                // Refresh system info
                let mut sys = System::new_all();
                sys.refresh_all();

                // If the cached PID is not running, try to find the current PID of the element.
                if sys.process(Pid::from(pid)).is_none() {
                    println!(
                        "Warning: Cached PID {} is no longer running. Searching for current process...",
                        pid
                    );

                    if let Some(new_pid) = get_process_listening_on_port(element.port) {
                        println!(
                            "Found element {} on port {} with PID: {}",
                            element.name.green(),
                            element.port,
                            new_pid
                        );
                        pid = new_pid;
                    }

                    if pid == 0 || sys.process(Pid::from(pid)).is_none() {
                        println!(
                            "Could not find a running process for command element {}. It may have already been stopped.",
                            element.name.red()
                        );
                        return;
                    }
                }

                match element.element_type.stop(pid, &mut state, &element.name) {
                    Ok(_) => { /* Success message already printed in stop() */ }
                    Err(e) => eprintln!(
                        "Failed to stop command element {}: {}",
                        element.name.red(),
                        e
                    ),
                }
            } else {
                println!(
                    "No record found for command element {}. It might not have been started with fastup or it was already closed.",
                    element.name.red()
                );
            }
        }
        ElementType::Service { .. } => {
            // Service -> Stop the service using systemctl
            match element.element_type.stop(0, &mut state, &element.name) {
                Ok(_) => { /* Success message already printed in stop() */ }
                Err(e) => eprintln!(
                    "Failed to stop service element {}: {}",
                    element.name.red(),
                    e
                ),
            }
        }
    }
}

/// Function to check if a specific port is open on the localhost
/// - `host`: Host to check
/// - `port`: Port to check
fn check_port(host: &str, port: u16) -> bool {
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
fn get_process_listening_on_port(port: u16) -> Option<usize> {
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
fn is_service_active(element_name: &str) -> bool {
    let status = Command::new("systemctl")
        .arg("is-active")
        .arg("--quiet")
        .arg(element_name)
        .status()
        .expect("Error occurred while checking element status");

    status.success()
}

/// Function to print the status of a element in a formatted way
/// - `name`: Name of the element
/// - `port`: Port of the element
/// - `online`: Whether the element is online or offline
fn print_status(name: &str, port: u16, online: bool) {
    // Format the status text with colors
    let status_text = if online {
        "ONLINE".on_green().white().bold()
    } else {
        "OFFLINE".on_red().white().bold()
    };

    // Print the element name, port, and status
    println!("{:<20} | Port: {:<5}| {}", name.blue(), port, status_text);
}
