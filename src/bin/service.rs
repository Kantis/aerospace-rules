use aerospace_rules::aerospace::list_windows_in_workspace;
use aerospace_rules::{aerospace, config, rules, Request, Response, ServiceState, SOCKET_PATH};
use clap::Parser;
use notify::{
    Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tokio::sync::RwLock;

#[derive(Parser)]
#[command(name = "aerospace-rules-service")]
#[command(about = "A service for managing aerospace window rules")]
struct Args {
    /// Path to config file
    #[arg(short, long)]
    config: Option<String>,
}

type SharedState = Arc<RwLock<ServiceState>>;

async fn handle_client(
    mut stream: UnixStream,
    state: SharedState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
                    match rules::evaluate_rules_for_workspace(
                        &workspace,
                        &state_guard.windows,
                        list_windows_in_workspace(workspace.as_str()).expect("foo"),
                        config,
                    ) {
                        Ok(actions) => Response::RulesEvaluated {
                            actions_performed: actions,
                        },
                        Err(e) => Response::Error(format!("Rule evaluation failed: {e}")),
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

fn get_config_file_path(explicit_path: Option<&str>) -> Option<PathBuf> {
    if let Some(path) = explicit_path {
        // Convert to absolute path
        let path_buf = PathBuf::from(path);
        if path_buf.is_absolute() {
            Some(path_buf)
        } else {
            // Make relative paths absolute by prepending current directory
            if let Ok(current_dir) = std::env::current_dir() {
                Some(current_dir.join(path_buf))
            } else {
                Some(path_buf)
            }
        }
    } else {
        // Use the same logic as config::find_config_file() but return the path even if file doesn't exist
        let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| format!("{}/.config", std::env::var("HOME").unwrap_or_default()));

        let xdg_path = PathBuf::from(xdg_runtime_dir)
            .join("aerospace")
            .join("rules.toml");

        if xdg_path.exists() {
            Some(xdg_path)
        } else if let Ok(home_dir) = std::env::var("HOME") {
            Some(PathBuf::from(home_dir).join(".aerospace-rules.toml"))
        } else {
            None
        }
    }
}

async fn refresh_state(state: SharedState) {
    println!("Refreshing aerospace state...");

    let windows = match aerospace::list_windows() {
        Ok(windows) => windows,
        Err(e) => {
            eprintln!("Failed to refresh windows: {e}");
            return;
        }
    };

    let config = {
        let state_guard = state.read().await;
        match &state_guard.config_path {
            Some(path) => config::load_config_from_path(Some(path)),
            None => config::load_config(),
        }
    };

    let mut state_guard = state.write().await;
    state_guard.windows = windows;
    state_guard.config = config;

    println!("State refreshed: {} windows", state_guard.windows.len());
}

async fn refresh_config_only(state: SharedState) {
    println!("Config file changed, reloading...");

    let config = {
        let state_guard = state.read().await;
        match &state_guard.config_path {
            Some(path) => config::load_config_from_path(Some(path)),
            None => config::load_config(),
        }
    };

    let mut state_guard = state.write().await;
    state_guard.config = config;

    match &state_guard.config {
        Some(config) => println!("Config reloaded successfully: {} rules", config.rules.len()),
        None => println!("Config file not found or invalid"),
    }
}

async fn watch_config_file(
    config_path: PathBuf,
    state: SharedState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (tx, mut rx) = mpsc::unbounded_channel();

    // We need to watch the parent directory since the file might not exist initially

    let mut watcher = RecommendedWatcher::new(
        move |result: Result<Event, notify::Error>| match result {
            Ok(event) => {
                if let Err(e) = tx.send(event) {
                    eprintln!("Failed to send watch event: {e}");
                }
            }
            Err(e) => eprintln!("Watch error: {e}"),
        },
        NotifyConfig::default(),
    )?;

    // Watch the directory containing the config file
    if let Some(parent_dir) = config_path.parent() {
        // Ensure the parent directory exists
        if let Err(e) = std::fs::create_dir_all(parent_dir) {
            eprintln!("Failed to create config directory {parent_dir:?}: {e}");
        }

        if let Err(e) = watcher.watch(parent_dir, RecursiveMode::NonRecursive) {
            eprintln!("Failed to watch config directory {parent_dir:?}: {e}");
            return Err(e.into());
        }
        println!("Watching config directory: {parent_dir:?}");
    }

    // Process filesystem events
    while let Some(event) = rx.recv().await {
        // Check if the event is related to our config file
        let relevant_event = event
            .paths
            .iter()
            .any(|path| path == &config_path || path.file_name() == config_path.file_name());

        if relevant_event {
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) => {
                    println!("Config file change detected: {:?}", event.kind);
                    refresh_config_only(state.clone()).await;
                }
                EventKind::Remove(_) => {
                    println!("Config file removed");
                    let mut state_guard = state.write().await;
                    state_guard.config = None;
                }
                _ => {
                    // Ignore other event types
                }
            }
        }
    }

    Ok(())
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
    let args = Args::parse();

    println!("Starting aerospace-rules service...");

    // Get config path for watching before moving args.config
    let config_path_for_watching = get_config_file_path(args.config.as_deref());

    // Initialize state
    let state = Arc::new(RwLock::new(ServiceState {
        windows: Vec::new(),
        config: None,
        config_path: args.config,
    }));

    // Initial state refresh
    refresh_state(state.clone()).await;

    // Start config file watcher if we have a config path to watch
    if let Some(config_path) = config_path_for_watching {
        let watcher_state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = watch_config_file(config_path, watcher_state).await {
                eprintln!("Config file watcher failed: {e}");
            }
        });
    } else {
        println!("No config file path available for watching");
    }

    // Start periodic refresh task
    let refresh_state = state.clone();
    tokio::spawn(async move {
        periodic_refresh(refresh_state).await;
    });

    // Remove existing socket file if it exists
    let _ = std::fs::remove_file(SOCKET_PATH);

    // Start Unix socket server
    let listener = UnixListener::bind(SOCKET_PATH)?;
    println!("Service listening on {SOCKET_PATH}");

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let state_clone = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, state_clone).await {
                        eprintln!("Error handling client: {e}");
                    }
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {e}");
            }
        }
    }
}
