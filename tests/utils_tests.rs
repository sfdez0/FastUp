use fastup::utils::{
    ensure_dir, get_config_dir, get_config_file, get_data_dir, get_log_file, get_logs_dir,
    get_state_file, strip_ansi_codes,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn expected_home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
}

fn unique_temp_dir() -> PathBuf {
    let mut dir = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    dir.push(format!("fastup_test_{}_{}", std::process::id(), nanos));
    dir
}

/// Test that validates strip_ansi_codes function
#[test]
fn test_strip_ansi_codes() {
    let input = "Hello \x1b[31mRed\x1b[0m World";
    let output = strip_ansi_codes(input);

    assert_eq!(output, "Hello Red World");
}

/// Test that validates that path helper functions use the home directory
#[test]
fn test_path_helpers() {
    let home = expected_home();

    assert_eq!(get_config_dir(), format!("{}/.config/fastup", home));
    assert_eq!(
        get_config_file(),
        format!("{}/.config/fastup/fastup.yaml", home)
    );
    assert_eq!(get_data_dir(), format!("{}/.local/share/fastup", home));
    assert_eq!(get_logs_dir(), format!("{}/.local/share/fastup/logs", home));
    assert_eq!(
        get_log_file(),
        format!("{}/.local/share/fastup/logs/fastup.txt", home)
    );
    assert_eq!(
        get_state_file(),
        format!("{}/.local/share/fastup/state.json", home)
    );
}

/// Test that ensure_dir creates directories as needed
#[test]
fn ensure_dir_creates_directory() {
    let dir = unique_temp_dir();
    let dir_str = dir.to_str().expect("temp dir path");

    ensure_dir(dir_str).expect("ensure_dir should succeed");
    assert!(dir.is_dir());

    fs::remove_dir_all(&dir).expect("cleanup temp dir");
}
