pub mod aerospace;
pub mod config;
pub mod rules;

pub use aerospace::WindowInfo;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    GetWindows,
    GetConfig,
    Reload,
    EvaluateRules { workspace: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Windows(Vec<WindowInfo>),
    Config(config::Config),
    Success,
    Error(String),
    RulesEvaluated { actions_performed: Vec<String> },
}

#[derive(Debug, Clone)]
pub struct ServiceState {
    pub windows: Vec<WindowInfo>,
    pub config: Option<config::Config>,
    pub config_path: Option<String>,
}

pub const SOCKET_PATH: &str = "/tmp/aerospace-rules.sock";
