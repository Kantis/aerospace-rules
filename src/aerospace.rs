use serde::Deserialize;
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

fn list_workspaces() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let output = Command::new("aerospace")
        .args(&["list-workspaces", "--all"])
        .output()?;

    if !output.status.success() {
        return Err(format!("aerospace list-workspaces failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }

    let workspaces_str = String::from_utf8(output.stdout)?;
    let workspaces: Vec<String> = workspaces_str
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    Ok(workspaces)
}

fn list_windows_in_workspace(workspace: &str) -> Result<Vec<AerospaceWindow>, Box<dyn std::error::Error>> {
    let output = Command::new("aerospace")
        .args(&["list-windows", "--workspace", workspace, "--json"])
        .output()?;

    if !output.status.success() {
        return Err(format!("aerospace list-windows for workspace {} failed: {}", workspace, String::from_utf8_lossy(&output.stderr)).into());
    }

    let json_str = String::from_utf8(output.stdout)?;
    let aerospace_windows: Vec<AerospaceWindow> = serde_json::from_str(&json_str)?;
    
    Ok(aerospace_windows)
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