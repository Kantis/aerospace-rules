use aerospace_rules::{aerospace, config, Request, Response, SOCKET_PATH};
use clap::Parser;
use std::env;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

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

#[derive(Parser)]
#[command(name = "aerospace-rules")]
#[command(about = "A CLI client for aerospace window rules")]
struct Args {
    /// Path to config file
    #[arg(short, long)]
    config: Option<String>,

    /// Command to execute
    #[arg(value_enum)]
    command: Option<Command>,
}

#[derive(clap::ValueEnum, Clone)]
enum Command {
    Windows,
    Config,
    Reload,
    OnWorkspaceChange,
}

async fn fallback_direct(config_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Service unavailable, falling back to direct queries...");

    match config::load_config_from_path(config_path) {
        Some(config) => {
            println!("Loaded {} rules", config.rules.len());
            for rule in &config.rules {
                match &rule.rule_type {
                    config::RuleType::Window { condition, .. } => {
                        println!("Rule: {} - {}", rule.name, condition);
                    }
                    config::RuleType::EmptyWorkspace { workspace, command } => {
                        println!(
                            "Rule: {} - empty workspace {} -> {}",
                            rule.name, workspace, command
                        );
                    }
                }
            }
        }
        None => println!("No config file found, running with defaults"),
    }

    match aerospace::list_windows() {
        Ok(windows) => {
            println!("\nFound {} windows:", windows.len());
            for window in &windows {
                println!(
                    "  [{}] {} (ID: {}) - {}",
                    window.workspace, window.app_name, window.window_id, window.window_title
                );
            }
        }
        Err(e) => println!("Failed to list windows: {e}"),
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Handle legacy command line arguments for backwards compatibility
    let command = if let Some(cmd) = args.command {
        cmd
    } else {
        let legacy_args: Vec<String> = env::args().collect();
        match legacy_args.get(1).map(|s| s.as_str()).unwrap_or("windows") {
            "windows" => Command::Windows,
            "config" => Command::Config,
            "reload" => Command::Reload,
            "on-workspace-change" => Command::OnWorkspaceChange,
            _ => {
                eprintln!(
                    "Usage: {} [--config <path>] [windows|config|reload|on-workspace-change]",
                    legacy_args[0]
                );
                return Ok(());
            }
        }
    };

    let request = match command {
        Command::Windows => Request::GetWindows,
        Command::Config => Request::GetConfig,
        Command::Reload => Request::Reload,
        Command::OnWorkspaceChange => {
            let workspace = env::var("AEROSPACE_FOCUSED_WORKSPACE")
                .map_err(|_| "AEROSPACE_FOCUSED_WORKSPACE environment variable not set")?;
            Request::EvaluateRules { workspace }
        }
    };

    match query_service(request).await {
        Ok(response) => match response {
            Response::Windows(windows) => {
                println!("Found {} windows:", windows.len());
                for window in &windows {
                    println!(
                        "  [{}] {} (ID: {}) - {}",
                        window.workspace, window.app_name, window.window_id, window.window_title
                    );
                }
            }
            Response::Config(config) => {
                println!("Loaded {} rules", config.rules.len());
                for rule in &config.rules {
                    match &rule.rule_type {
                        config::RuleType::Window { condition, .. } => {
                            println!("Rule: {} - {}", rule.name, condition);
                        }
                        config::RuleType::EmptyWorkspace { workspace, command } => {
                            println!(
                                "Rule: {} - empty workspace {} -> {}",
                                rule.name, workspace, command
                            );
                        }
                    }
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
                        println!("  {action}");
                    }
                }
            }
            Response::Error(err) => {
                eprintln!("Service error: {err}");
            }
        },
        Err(e) => {
            eprintln!("Failed to connect to service: {e}");
            if matches!(command, Command::Windows) {
                fallback_direct(args.config.as_deref()).await?;
            }
        }
    }

    Ok(())
}
