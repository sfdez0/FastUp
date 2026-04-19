use serde::Deserialize;
use std::fs::{self};

use crate::elements::ElementConfig;
use crate::elements::ElementType;
use crate::state::FastUpState;
use crate::utils::check_port;
use crate::utils::get_process_listening_on_port;
use crate::utils::is_service_active;
use crate::warn;

/// Path to the config file
pub const CONFIG_FILE: &str = "config/fastup.yaml";

/// Struct to represent the configuration of the application as defined in the YAML config file
#[derive(Deserialize)]
pub struct FastUpConfig {
    /// List of elements defined in the config file
    pub elements_config: Vec<ElementConfig>,
}

/// Function to load the configuration from the YAML file
pub fn load_config() -> FastUpConfig {
    let content = fs::read_to_string(CONFIG_FILE).expect("Could not find the fastup.yaml file");

    serde_yml::from_str(&content).expect("Error parsing the fastup.yaml file")
}

/// Function to refresh the status of the elements by cleaning up dead elements from the state and checking for externally running elements
/// It also returns the loaded configuration.
pub fn refresh_status() -> FastUpConfig {
    // Load the config and the current state
    let config = load_config();
    let mut state = FastUpState::load();

    // Clean up dead elements from state and save
    if let Err(e) = state.cleanup_dead_elements() {
        warn!("Failed to update element state: {}", e);
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
                warn!(
                    "Failed to register external element {}: {}",
                    element.name, e
                );
            }
        }
    }

    config
}
