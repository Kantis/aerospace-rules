mod aerospace;
mod config;

use aerospace::list_windows;
use config::load_config;

fn main() {
    println!("Hello, world!");

    match load_config() {
        Some(config) => {
            println!("Loaded {} rules", config.rules.len());
            for rule in &config.rules {
                match &rule.rule_type {
                    config::RuleType::Window { condition, .. } => {
                        println!("Rule: {} - {}", rule.name, condition);
                    }
                    config::RuleType::EmptyWorkspace { workspace, command } => {
                        println!(
                            "Rule: {} - empty workspace {} -> {}",
                            rule.name, workspace, command
                        );
                    }
                }
            }
        }
        None => println!("No config file found, running with defaults"),
    }

    match list_windows() {
        Ok(windows) => {
            println!("\nFound {} windows:", windows.len());
            for window in &windows {
                println!(
                    "  [{}] {} (ID: {}) - {}",
                    window.workspace, window.app_name, window.window_id, window.window_title
                );
            }
        }
        Err(e) => println!("Failed to list windows: {}", e),
    }
}
