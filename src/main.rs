use serde::Deserialize;
use std::fs;
use std::env;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
struct Config {
    rules: Vec<Rule>,
}

#[derive(Deserialize, Debug)]
struct Rule {
    name: String,
    condition: String,
    action: String,
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

fn main() {
    println!("Hello, world!");
    
    match find_config_file() {
        Some(config_path) => {
            match fs::read_to_string(&config_path) {
                Ok(config_content) => {
                    match toml::from_str::<Config>(&config_content) {
                        Ok(config) => {
                            println!("Loaded {} rules from {}", config.rules.len(), config_path.display());
                            for rule in &config.rules {
                                println!("Rule: {} - {}", rule.name, rule.condition);
                            }
                        }
                        Err(e) => println!("Failed to parse {}: {}", config_path.display(), e),
                    }
                }
                Err(e) => println!("Failed to read {}: {}", config_path.display(), e),
            }
        }
        None => println!("No config file found at $XDG_RUNTIME_DIR/aerospace/rules.toml or $HOME/.aerospace-rules.toml, running with defaults"),
    }
}
