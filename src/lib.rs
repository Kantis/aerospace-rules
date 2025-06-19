pub mod config;
pub mod aerospace;

use serde::{Deserialize, Serialize};
pub use aerospace::WindowInfo;

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    GetWindows,
    GetConfig,
    Reload,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Windows(Vec<WindowInfo>),
    Config(config::Config),
    Success,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct ServiceState {
    pub windows: Vec<WindowInfo>,
    pub config: Option<config::Config>,
}

pub const SOCKET_PATH: &str = "/tmp/aerospace-rules.sock";