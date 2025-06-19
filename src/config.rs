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
    let config_path = find_config_file()?;
    let config_content = fs::read_to_string(&config_path).ok()?;
    toml::from_str::<Config>(&config_content).ok()
}