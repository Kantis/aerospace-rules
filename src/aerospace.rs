use serde::Deserialize;
use std::error::Error;
use std::process::Command;

#[derive(serde::Serialize, Deserialize, Debug, Clone)]
pub struct WindowInfo {
    #[serde(rename = "app-name")]
    pub app_name: String,
    #[serde(rename = "window-id")]
    pub window_id: u32,
    #[serde(rename = "window-title")]
    pub window_title: String,
    pub workspace: String,
}

#[derive(Deserialize)]
struct AerospaceWindow {
    #[serde(rename = "app-name")]
    app_name: String,
    #[serde(rename = "window-id")]
    window_id: u32,
    #[serde(rename = "window-title")]
    window_title: String,
}

fn execute_command(args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("aerospace").args(args).output()?;

    if !output.status.success() {
        return Err(format!(
            "aerospace list-workspaces failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(String::from_utf8(output.stdout)?)
}

fn list_workspaces() -> Result<Vec<String>, Box<dyn Error>> {
    execute_command(&["list-workspaces", "--all"]).map(|s| {
        s.lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect()
    })
}

fn list_windows_in_workspace(workspace: &str) -> Result<Vec<AerospaceWindow>, Box<dyn Error>> {
    execute_command(&["list-windows", "--workspace", workspace, "--json"])
        .map(|s| serde_json::from_str::<Vec<AerospaceWindow>>(&s).map_err(|e| e.into())).flatten()
}

pub fn list_windows() -> Result<Vec<WindowInfo>, Box<dyn std::error::Error>> {
    let workspaces = list_workspaces()?;
    let mut all_windows = Vec::new();

    for workspace in workspaces {
        let workspace_windows = list_windows_in_workspace(&workspace)?;
        for window in workspace_windows {
            all_windows.push(WindowInfo {
                app_name: window.app_name,
                window_id: window.window_id,
                window_title: window.window_title,
                workspace: workspace.clone(),
            });
        }
    }

    Ok(all_windows)
}
