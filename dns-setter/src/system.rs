//! Platform-specific helpers for interacting with Windows networking.

use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

use regex::Regex;

use crate::domain::OperationResult;

const CREATE_NO_WINDOW: u32 = 0x0800_0000; // Hide console window

/// Run `netsh` with the provided arguments.
fn run_netsh(args: &[&str]) -> std::process::Output {
    Command::new("netsh")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to run netsh")
        .wait_with_output()
        .expect("Failed to wait for netsh")
}

/// Get the currently active adapter name.
pub fn get_active_adapter() -> Option<String> {
    let output = run_netsh(&["interface", "show", "interface"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        if line.contains("Connected") && line.contains("Dedicated") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            return parts.last().map(|s| s.to_string());
        }
    }
    None
}

/// Return DNS servers currently configured for the adapter.
pub fn get_current_dns(adapter: &str) -> Vec<String> {
    let output = run_netsh(&[
        "interface",
        "ip",
        "show",
        "dns",
        &format!("name={}", adapter),
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    let re = Regex::new(r"\b\d{1,3}(?:\.\d{1,3}){3}\b").unwrap();
    re.find_iter(&stdout)
        .map(|m| m.as_str().to_string())
        .collect()
}

/// Set DNS servers and return a result suitable for UI consumption.
pub fn set_dns_with_result(interface: &str, primary: &str, secondary: &str) -> OperationResult {
    let output1 = run_netsh(&[
        "interface",
        "ipv4",
        "set",
        "dns",
        &format!("name={}", interface),
        "static",
        primary,
    ]);

    if !output1.status.success() {
        return OperationResult::Error(format!(
            "Error setting primary DNS {}: {}",
            primary,
            String::from_utf8_lossy(&output1.stderr)
        ));
    }

    let output2 = run_netsh(&[
        "interface",
        "ipv4",
        "add",
        "dns",
        &format!("name={}", interface),
        secondary,
        "index=2",
    ]);

    if !output2.status.success() {
        return OperationResult::Error(format!(
            "Error setting secondary DNS {}: {}",
            secondary,
            String::from_utf8_lossy(&output2.stderr)
        ));
    }

    OperationResult::Success(format!(
        "DNS servers {} and {} set successfully for '{}'",
        primary, secondary, interface
    ))
}

/// Clear DNS and return a UI-friendly result.
pub fn clear_dns_with_result(interface: &str) -> OperationResult {
    let output = run_netsh(&[
        "interface",
        "ipv4",
        "set",
        "dns",
        &format!("name={}", interface),
        "source=dhcp",
    ]);
    if output.status.success() {
        OperationResult::Success(format!(
            "DNS reset to DHCP successfully for '{}'",
            interface
        ))
    } else {
        OperationResult::Error(format!(
            "Error resetting DNS for '{}': {}",
            interface,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}
