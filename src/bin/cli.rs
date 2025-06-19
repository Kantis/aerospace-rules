use aerospace_rules::{config, aerospace, Request, Response, SOCKET_PATH};
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::env;

async fn query_service(request: Request) -> Result<Response, Box<dyn std::error::Error>> {
    let mut stream = UnixStream::connect(SOCKET_PATH).await?;
    
    let request_json = serde_json::to_string(&request)?;
    stream.write_all(request_json.as_bytes()).await?;
    
    let mut buffer = vec![0; 8192];
    let n = stream.read(&mut buffer).await?;
    
    let response_str = String::from_utf8_lossy(&buffer[..n]);
    let response: Response = serde_json::from_str(&response_str)?;
    
    Ok(response)
}

async fn fallback_direct() -> Result<(), Box<dyn std::error::Error>> {
    println!("Service unavailable, falling back to direct queries...");
    
    match config::load_config() {
        Some(config) => {
            println!("Loaded {} rules", config.rules.len());
            for rule in &config.rules {
                println!("Rule: {} - {}", rule.name, rule.condition);
            }
        }
        None => println!("No config file found, running with defaults"),
    }
    
    match aerospace::list_windows() {
        Ok(windows) => {
            println!("\nFound {} windows:", windows.len());
            for window in &windows {
                println!("  [{}] {} (ID: {}) - {}", window.workspace, window.app_name, window.window_id, window.window_title);
            }
        }
        Err(e) => println!("Failed to list windows: {}", e),
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("windows");
    
    let request = match command {
        "windows" => Request::GetWindows,
        "config" => Request::GetConfig,
        "reload" => Request::Reload,
        "on-workspace-change" => {
            let workspace = args.get(2).ok_or("Usage: aerospace-rules-cli on-workspace-change <workspace>")?;
            Request::EvaluateRules { workspace: workspace.to_string() }
        }
        _ => {
            eprintln!("Usage: {} [windows|config|reload|on-workspace-change <workspace>]", args[0]);
            return Ok(());
        }
    };
    
    match query_service(request).await {
        Ok(response) => {
            match response {
                Response::Windows(windows) => {
                    println!("Found {} windows:", windows.len());
                    for window in &windows {
                        println!("  [{}] {} (ID: {}) - {}", window.workspace, window.app_name, window.window_id, window.window_title);
                    }
                }
                Response::Config(config) => {
                    println!("Loaded {} rules", config.rules.len());
                    for rule in &config.rules {
                        println!("Rule: {} - {}", rule.name, rule.condition);
                    }
                }
                Response::Success => {
                    println!("Command executed successfully");
                }
                Response::RulesEvaluated { actions_performed } => {
                    if actions_performed.is_empty() {
                        println!("No rules matched for workspace change");
                    } else {
                        println!("Rules evaluated successfully:");
                        for action in actions_performed {
                            println!("  {}", action);
                        }
                    }
                }
                Response::Error(err) => {
                    eprintln!("Service error: {}", err);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to service: {}", e);
            if command == "windows" {
                fallback_direct().await?;
            }
        }
    }
    
    Ok(())
}