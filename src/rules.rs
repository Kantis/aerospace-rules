use crate::{
    config::{Config, RuleType},
    WindowInfo,
};
use std::error::Error;
use std::process::Command;

pub fn evaluate_rules_for_workspace(
    workspace: &str,
    _windows: &[WindowInfo],
    focused_workspace_windows: Vec<WindowInfo>,
    config: &Config,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut actions_performed = Vec::new();

    println!(
        "Evaluating {} rules for workspace {workspace}",
        config.rules.len()
    );
    println!(
        "Found {} windows in workspace {workspace}",
        focused_workspace_windows.len(),
    );

    for rule in &config.rules {
        println!("Checking rule: {}", rule.name);

        match &rule.rule_type {
            RuleType::Window { condition, action } => {
                // Only process window rules if there are windows in the workspace
                if !focused_workspace_windows.is_empty() {
                    for window in &focused_workspace_windows {
                        if matches_condition(condition, window)? {
                            println!(
                                "Rule '{}' matches window: {} ({})",
                                rule.name, window.app_name, window.window_id,
                            );

                            if let Err(e) = execute_action(action, window) {
                                eprintln!(
                                    "Failed to execute action '{action}' for window {}: {e}",
                                    window.window_id,
                                );
                                continue;
                            }

                            actions_performed.push(format!(
                                "Applied '{}' to {} (ID: {}): {action}",
                                rule.name, window.app_name, window.window_id,
                            ));
                        }
                    }
                }
            }
            RuleType::EmptyWorkspace {
                workspace: rule_workspace,
                command,
            } => {
                // Only process empty workspace rules if workspace is empty and matches
                if focused_workspace_windows.is_empty() && rule_workspace == workspace {
                    println!("Workspace {workspace} is empty, executing command: {command}");

                    if let Err(e) = execute_empty_workspace_command(command) {
                        eprintln!("Failed to execute empty workspace command '{command}': {e}");
                        actions_performed.push(format!(
                            "Failed to execute empty workspace command '{}': {e}",
                            rule.name,
                        ));
                    } else {
                        actions_performed.push(format!(
                            "Executed empty workspace rule '{}': {command}",
                            rule.name,
                        ));
                    }
                }
            }
        }
    }

    Ok(actions_performed)
}

fn matches_condition(condition: &str, window: &WindowInfo) -> Result<bool, Box<dyn Error>> {
    // Simple condition parser for now
    // Format: "field = 'value'" or "field > number"

    if condition.contains(" = ") {
        let parts: Vec<&str> = condition.split(" = ").collect();
        if parts.len() != 2 {
            return Err(format!("Invalid condition format: {condition}").into());
        }

        let field = parts[0].trim();
        let value = parts[1].trim().trim_matches('\'').trim_matches('"');

        match field {
            "app-id" | "app-name" => Ok(window.app_name == value),
            "window-title" => Ok(window.window_title.contains(value)),
            "workspace" => Ok(window.workspace == value),
            _ => Err(format!("Unknown field in condition: {field}").into()),
        }
    } else if condition.contains(" > ") {
        let parts: Vec<&str> = condition.split(" > ").collect();
        if parts.len() != 2 {
            return Err(format!("Invalid condition format: {condition}").into());
        }

        let field = parts[0].trim();
        let value: u32 = parts[1].trim().parse()?;

        match field {
            "window-width" => {
                // For now, we'll assume all windows are "large" (> 1000)
                // In a real implementation, we'd query the actual window dimensions
                Ok(value < 1200) // Mock logic
            }
            "window-id" => Ok(window.window_id > value),
            _ => Err(format!("Unknown numeric field in condition: {field}").into()),
        }
    } else {
        Err(format!("Unsupported condition format: {condition}").into())
    }
}

fn execute_action(action: &str, window: &WindowInfo) -> Result<(), Box<dyn Error>> {
    println!(
        "Executing action: {} for window {}",
        action, window.window_id
    );

    if action.starts_with("move-to-workspace ") {
        let target_workspace = action.strip_prefix("move-to-workspace ").unwrap();

        let output = Command::new("aerospace")
            .args([
                "move",
                "--window-id",
                &window.window_id.to_string(),
                "--workspace",
                target_workspace,
            ])
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to move window to workspace {}: {}",
                target_workspace,
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        println!(
            "Moved window {} to workspace {}",
            window.window_id, target_workspace
        );
    } else if action == "maximize" {
        let output = Command::new("aerospace")
            .args(["fullscreen", "--window-id", &window.window_id.to_string()])
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to maximize window: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        println!("Maximized window {}", window.window_id);
    } else {
        return Err(format!("Unknown action: {action}").into());
    }

    Ok(())
}

fn execute_empty_workspace_command(command: &str) -> Result<(), Box<dyn Error>> {
    println!("Executing empty workspace command: {command}");

    // Parse command and arguments
    let parts = match shlex::split(command) {
        Some(parts) => parts,
        None => return Err(format!("Failed to parse command: {command}").into()),
    };

    if parts.is_empty() {
        return Err("Empty command".into());
    }

    let program = &parts[0];
    let args = &parts[1..];

    let output = Command::new(program).args(args).output()?;

    if !output.status.success() {
        return Err(format!(
            "Command '{command}' failed with exit code {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    // Log stdout if there's any output
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        println!("Command output: {}", stdout.trim());
    }

    println!("Successfully executed empty workspace command: {command}");
    Ok(())
}
