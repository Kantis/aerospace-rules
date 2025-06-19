use crate::{config::Config, WindowInfo};
use std::process::Command;

pub fn evaluate_rules_for_workspace(
    workspace: &str,
    windows: &[WindowInfo],
    config: &Config,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut actions_performed = Vec::new();
    
    // Get windows in the specified workspace
    let workspace_windows: Vec<&WindowInfo> = windows
        .iter()
        .filter(|w| w.workspace == workspace)
        .collect();
    
    println!("Evaluating {} rules for workspace {}", config.rules.len(), workspace);
    println!("Found {} windows in workspace {}", workspace_windows.len(), workspace);
    
    for rule in &config.rules {
        println!("Checking rule: {}", rule.name);
        
        for window in &workspace_windows {
            if matches_condition(&rule.condition, window)? {
                println!("Rule '{}' matches window: {} ({})", rule.name, window.app_name, window.window_id);
                
                if let Err(e) = execute_action(&rule.action, window) {
                    eprintln!("Failed to execute action '{}' for window {}: {}", rule.action, window.window_id, e);
                    continue;
                }
                
                actions_performed.push(format!(
                    "Applied '{}' to {} (ID: {}): {}",
                    rule.name, window.app_name, window.window_id, rule.action
                ));
            }
        }
    }
    
    Ok(actions_performed)
}

fn matches_condition(condition: &str, window: &WindowInfo) -> Result<bool, Box<dyn std::error::Error>> {
    // Simple condition parser for now
    // Format: "field = 'value'" or "field > number"
    
    if condition.contains(" = ") {
        let parts: Vec<&str> = condition.split(" = ").collect();
        if parts.len() != 2 {
            return Err(format!("Invalid condition format: {}", condition).into());
        }
        
        let field = parts[0].trim();
        let value = parts[1].trim().trim_matches('\'').trim_matches('"');
        
        match field {
            "app-id" | "app-name" => Ok(window.app_name == value),
            "window-title" => Ok(window.window_title.contains(value)),
            "workspace" => Ok(window.workspace == value),
            _ => Err(format!("Unknown field in condition: {}", field).into()),
        }
    } else if condition.contains(" > ") {
        let parts: Vec<&str> = condition.split(" > ").collect();
        if parts.len() != 2 {
            return Err(format!("Invalid condition format: {}", condition).into());
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
            _ => Err(format!("Unknown numeric field in condition: {}", field).into()),
        }
    } else {
        Err(format!("Unsupported condition format: {}", condition).into())
    }
}

fn execute_action(action: &str, window: &WindowInfo) -> Result<(), Box<dyn std::error::Error>> {
    println!("Executing action: {} for window {}", action, window.window_id);
    
    if action.starts_with("move-to-workspace ") {
        let target_workspace = action.strip_prefix("move-to-workspace ").unwrap();
        
        let output = Command::new("aerospace")
            .args(&["move", "--window-id", &window.window_id.to_string(), "--workspace", target_workspace])
            .output()?;
        
        if !output.status.success() {
            return Err(format!(
                "Failed to move window to workspace {}: {}",
                target_workspace,
                String::from_utf8_lossy(&output.stderr)
            ).into());
        }
        
        println!("Moved window {} to workspace {}", window.window_id, target_workspace);
    } else if action == "maximize" {
        let output = Command::new("aerospace")
            .args(&["fullscreen", "--window-id", &window.window_id.to_string()])
            .output()?;
        
        if !output.status.success() {
            return Err(format!(
                "Failed to maximize window: {}",
                String::from_utf8_lossy(&output.stderr)
            ).into());
        }
        
        println!("Maximized window {}", window.window_id);
    } else {
        return Err(format!("Unknown action: {}", action).into());
    }
    
    Ok(())
}