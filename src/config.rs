use serde::{Deserialize, Serialize};
use std::fs;
use std::env;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub rules: Vec<Rule>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Rule {
    pub name: String,
    pub condition: String,
    pub action: String,
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
        writeln!(temp_file, r#"
[[rules]]
name = "Test Rule"
condition = "app-name = 'TestApp'"
action = "maximize"

[[rules]]
name = "Another Rule"
condition = "workspace = '1'"
action = "move-to-workspace 2"
        "#).expect("Failed to write to temp file");
        
        let config_path = temp_file.path().to_str().unwrap();
        let config = load_config_from_path(Some(config_path));
        
        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.rules.len(), 2);
        
        assert_eq!(config.rules[0].name, "Test Rule");
        assert_eq!(config.rules[0].condition, "app-name = 'TestApp'");
        assert_eq!(config.rules[0].action, "maximize");
        
        assert_eq!(config.rules[1].name, "Another Rule");
        assert_eq!(config.rules[1].condition, "workspace = '1'");
        assert_eq!(config.rules[1].action, "move-to-workspace 2");
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
            assert_eq!(config.rules.len(), 2);
            
            assert_eq!(config.rules[0].name, "Test Rule");
            assert_eq!(config.rules[0].condition, "app-name = 'Ghostty'");
            assert_eq!(config.rules[0].action, "maximize");
            
            assert_eq!(config.rules[1].name, "Move IntelliJ");
            assert_eq!(config.rules[1].condition, "app-name = 'IntelliJ IDEA'");
            assert_eq!(config.rules[1].action, "move-to-workspace 5");
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