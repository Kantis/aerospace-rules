use aerospace_rules::{config, aerospace, rules, Request, Response, ServiceState, SOCKET_PATH};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::time::Duration;

type SharedState = Arc<RwLock<ServiceState>>;

async fn handle_client(mut stream: UnixStream, state: SharedState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut buffer = vec![0; 1024];
    let n = stream.read(&mut buffer).await?;
    
    if n == 0 {
        return Ok(());
    }
    
    let request_str = String::from_utf8_lossy(&buffer[..n]);
    let request: Request = serde_json::from_str(&request_str)?;
    
    let response = match request {
        Request::GetWindows => {
            let state_guard = state.read().await;
            Response::Windows(state_guard.windows.clone())
        }
        Request::GetConfig => {
            let state_guard = state.read().await;
            match &state_guard.config {
                Some(config) => Response::Config(config.clone()),
                None => Response::Error("No config loaded".to_string()),
            }
        }
        Request::Reload => {
            refresh_state(state.clone()).await;
            Response::Success
        }
        Request::EvaluateRules { workspace } => {
            let state_guard = state.read().await;
            match &state_guard.config {
                Some(config) => {
                    match rules::evaluate_rules_for_workspace(&workspace, &state_guard.windows, config) {
                        Ok(actions) => Response::RulesEvaluated { actions_performed: actions },
                        Err(e) => Response::Error(format!("Rule evaluation failed: {}", e)),
                    }
                }
                None => Response::Error("No config loaded".to_string()),
            }
        }
    };
    
    let response_json = serde_json::to_string(&response)?;
    stream.write_all(response_json.as_bytes()).await?;
    
    Ok(())
}

async fn refresh_state(state: SharedState) {
    println!("Refreshing aerospace state...");
    
    let windows = match aerospace::list_windows() {
        Ok(windows) => windows,
        Err(e) => {
            eprintln!("Failed to refresh windows: {}", e);
            return;
        }
    };
    
    let config = config::load_config();
    
    let mut state_guard = state.write().await;
    state_guard.windows = windows;
    state_guard.config = config;
    
    println!("State refreshed: {} windows", state_guard.windows.len());
}

async fn periodic_refresh(state: SharedState) {
    let mut interval = tokio::time::interval(Duration::from_secs(2));
    
    loop {
        interval.tick().await;
        refresh_state(state.clone()).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting aerospace-rules service...");
    
    // Initialize state
    let state = Arc::new(RwLock::new(ServiceState {
        windows: Vec::new(),
        config: None,
    }));
    
    // Initial state refresh
    refresh_state(state.clone()).await;
    
    // Start periodic refresh task
    let refresh_state = state.clone();
    tokio::spawn(async move {
        periodic_refresh(refresh_state).await;
    });
    
    // Remove existing socket file if it exists
    let _ = std::fs::remove_file(SOCKET_PATH);
    
    // Start Unix socket server
    let listener = UnixListener::bind(SOCKET_PATH)?;
    println!("Service listening on {}", SOCKET_PATH);
    
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let state_clone = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, state_clone).await {
                        eprintln!("Error handling client: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }
}