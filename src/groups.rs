use serde::Deserialize;

/// Struct to represent a group of elements as defined in the YAML config file
#[derive(Deserialize, Clone)]
pub struct GroupConfig {
    /// Name of the group
    pub name: String,
    /// Description of the group
    pub description: Option<String>,
    /// List of element names in this group
    pub elements: Vec<String>,
}
