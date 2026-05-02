use fastup::config::FastUpConfig;
use fastup::elements::ElementType;

/// Test that validates YAML parsing of a simple configuration file
#[test]
fn test_parse_simple_config() {
    let yaml = r#"
elements_config:
  - name: "worker"
    port: 9000
    element_type: "Command"
    start_command: "node"
    args: 
      - "server.js"
groups_config:
"#;

    let config: FastUpConfig = yaml_serde::from_str(yaml).expect("valid config yaml");

    match &config.elements_config[0].element_type {
        ElementType::Command {
            start_command,
            log_file,
            ..
        } => {
            assert_eq!(start_command, "node");
            assert!(log_file.is_none());
        }
        _ => panic!("expected command element"),
    }
}

/// Test that validates YAML parsing of a complex configuration file
#[test]
fn test_parse_complex_config() {
    let yaml = r#"
elements_config:
  - name: "api"
    port: 8080
    element_type: "Command"
    start_command: "python"
    args: 
      - "-m"
      - "http.server"
      - "8080"
    log_file: "api.log"
  - name: "redis"
    port: 6379
    element_type: "Service"
    service_name: "redis"

groups_config:
  - name: "backend"
    description: "Backend services"
    elements: 
      - "api"
      - "redis"
"#;

    let config: FastUpConfig = yaml_serde::from_str(yaml).expect("valid config yaml");

    assert_eq!(config.elements_config.len(), 2);
    assert_eq!(config.groups_config.len(), 1);

    match &config.elements_config[0].element_type {
        ElementType::Command {
            start_command,
            args,
            log_file,
        } => {
            assert_eq!(start_command, "python");
            assert_eq!(args, &vec!["-m", "http.server", "8080"]);
            assert_eq!(log_file.as_deref(), Some("api.log"));
        }
        _ => panic!("expected command element"),
    }

    match &config.elements_config[1].element_type {
        ElementType::Service { service_name } => {
            assert_eq!(service_name, "redis");
        }
        _ => panic!("expected service element"),
    }

    assert_eq!(config.groups_config[0].name, "backend");
    assert_eq!(
        config.groups_config[0].description.as_deref(),
        Some("Backend services")
    );
    assert_eq!(config.groups_config[0].elements, vec!["api", "redis"]);
}
