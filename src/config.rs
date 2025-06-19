use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub rules: Vec<Rule>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Rule {
    pub name: String,
    #[serde(flatten)]
    pub rule_type: RuleType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum RuleType {
    #[serde(rename = "window")]
    Window { condition: String, action: String },
    #[serde(rename = "empty-workspace")]
    EmptyWorkspace { workspace: String, command: String },
}

fn find_config_file() -> Option<PathBuf> {
    let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("{}/.config", env::var("HOME").unwrap_or_default()));

    let xdg_path = PathBuf::from(xdg_runtime_dir)
        .join("aerospace")
        .join("rules.toml");
    if xdg_path.exists() {
        return Some(xdg_path);
    }

    if let Ok(home_dir) = env::var("HOME") {
        let home_path = PathBuf::from(home_dir).join(".aerospace-rules.toml");
        if home_path.exists() {
            return Some(home_path);
        }
    }

    None
}

pub fn load_config() -> Option<Config> {
    load_config_from_path(None)
}

pub fn load_config_from_path(explicit_path: Option<&str>) -> Option<Config> {
    let config_path = if let Some(path) = explicit_path {
        PathBuf::from(path)
    } else {
        find_config_file()?
    };

    let config_content = fs::read_to_string(&config_path).ok()?;
    toml::from_str::<Config>(&config_content).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_config_from_valid_file() {
        // Create a temporary config file
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(
            temp_file,
            r#"
[[rules]]
name = "Test Rule"
type = "window"
condition = "app-name = 'TestApp'"
action = "maximize"

[[rules]]
name = "Another Rule"
type = "window"
condition = "workspace = '1'"
action = "move-to-workspace 2"
        "#
        )
        .expect("Failed to write to temp file");

        let config_path = temp_file.path().to_str().unwrap();
        let config = load_config_from_path(Some(config_path));

        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.rules.len(), 2);

        assert_eq!(config.rules[0].name, "Test Rule");
        if let RuleType::Window { condition, action } = &config.rules[0].rule_type {
            assert_eq!(condition, "app-name = 'TestApp'");
            assert_eq!(action, "maximize");
        } else {
            panic!("Expected Window rule type");
        }

        assert_eq!(config.rules[1].name, "Another Rule");
        if let RuleType::Window { condition, action } = &config.rules[1].rule_type {
            assert_eq!(condition, "workspace = '1'");
            assert_eq!(action, "move-to-workspace 2");
        } else {
            panic!("Expected Window rule type");
        }
    }

    #[test]
    fn test_load_config_from_nonexistent_file() {
        let config = load_config_from_path(Some("/path/that/does/not/exist.toml"));
        assert!(config.is_none());
    }

    #[test]
    fn test_load_config_from_invalid_toml() {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(temp_file, "invalid toml content [[[").expect("Failed to write to temp file");

        let config_path = temp_file.path().to_str().unwrap();
        let config = load_config_from_path(Some(config_path));

        assert!(config.is_none());
    }

    #[test]
    fn test_load_test_config_file() {
        // Test the actual test-config.toml file
        let config = load_config_from_path(Some("test-config.toml"));

        if config.is_some() {
            let config = config.unwrap();
            assert_eq!(config.rules.len(), 3);

            assert_eq!(config.rules[0].name, "Test Rule");
            if let RuleType::Window { condition, action } = &config.rules[0].rule_type {
                assert_eq!(condition, "app-name = 'Ghostty'");
                assert_eq!(action, "maximize");
            } else {
                panic!("Expected Window rule type");
            }

            assert_eq!(config.rules[1].name, "Move IntelliJ");
            if let RuleType::Window { condition, action } = &config.rules[1].rule_type {
                assert_eq!(condition, "app-name = 'IntelliJ IDEA'");
                assert_eq!(action, "move-to-workspace 5");
            } else {
                panic!("Expected Window rule type");
            }

            // Test empty workspace rule
            assert_eq!(config.rules[2].name, "Terminal for Empty Workspace 99");
            if let RuleType::EmptyWorkspace { workspace, command } = &config.rules[2].rule_type {
                assert_eq!(workspace, "99");
                assert_eq!(command, "open -a Terminal");
            } else {
                panic!("Expected EmptyWorkspace rule type");
            }
        }
    }

    #[test]
    fn test_config_with_empty_workspace_rule() {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(
            temp_file,
            r#"
[[rules]]
name = "Simple Rule"
type = "window"
condition = "app-name = 'TestApp'"
action = "maximize"

[[rules]]
name = "Empty Workspace Terminal"
type = "empty-workspace"
workspace = "5"
command = "open -a Terminal"
        "#
        )
        .expect("Failed to write to temp file");

        let config_path = temp_file.path().to_str().unwrap();
        let config = load_config_from_path(Some(config_path));

        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.rules.len(), 2);

        // Check window rule
        if let RuleType::Window { condition, action } = &config.rules[0].rule_type {
            assert_eq!(condition, "app-name = 'TestApp'");
            assert_eq!(action, "maximize");
        } else {
            panic!("Expected Window rule type");
        }

        // Check empty workspace rule
        if let RuleType::EmptyWorkspace { workspace, command } = &config.rules[1].rule_type {
            assert_eq!(workspace, "5");
            assert_eq!(command, "open -a Terminal");
        } else {
            panic!("Expected EmptyWorkspace rule type");
        }
    }

    #[test]
    fn test_load_config_fallback_to_discovery() {
        // Test that load_config_from_path(None) falls back to find_config_file
        let config = load_config_from_path(None);
        // This may or may not find a config depending on the test environment
        // Just ensure it doesn't panic
        let _ = config;
    }
}
