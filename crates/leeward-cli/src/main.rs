//! leeward CLI - Command line interface for the sandbox

use clap::{Parser, Subcommand};
use leeward_core::config::default_socket_path;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "leeward")]
#[command(author, version, about = "Linux-native sandbox for untrusted code execution")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute Python code
    Exec {
        /// Code to execute (or - for stdin)
        code: String,

        /// Socket path (defaults to LEEWARD_SOCKET env var or /run/leeward/leeward.sock)
        #[arg(short, long)]
        socket: Option<PathBuf>,

        /// Timeout in seconds
        #[arg(short, long, default_value = "30")]
        timeout: u64,

        /// Memory limit in MB
        #[arg(short, long, default_value = "256")]
        memory: u64,
    },

    /// Get daemon status
    Status {
        /// Socket path (defaults to LEEWARD_SOCKET env var or /run/leeward/leeward.sock)
        #[arg(short, long)]
        socket: Option<PathBuf>,
    },

    /// Ping the daemon
    Ping {
        /// Socket path (defaults to LEEWARD_SOCKET env var or /run/leeward/leeward.sock)
        #[arg(short, long)]
        socket: Option<PathBuf>,
    },

    /// Run code directly (without daemon, for testing)
    Run {
        /// Code to execute
        code: String,

        /// Timeout in seconds
        #[arg(short, long, default_value = "30")]
        timeout: u64,

        /// Memory limit in MB
        #[arg(short, long, default_value = "256")]
        memory: u64,

        /// Allow network access
        #[arg(long)]
        network: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("leeward=info".parse()?),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Exec {
            code,
            socket,
            timeout,
            memory,
        } => {
            let socket = socket.unwrap_or_else(default_socket_path);
            println!("Executing via daemon at {:?}", socket);
            println!("Code: {}", code);
            println!("Timeout: {}s, Memory: {}MB", timeout, memory);
            // TODO: Connect to daemon and execute
        }

        Commands::Status { socket } => {
            let socket = socket.unwrap_or_else(default_socket_path);
            println!("Getting status from {:?}", socket);
            // TODO: Connect to daemon and get status
        }

        Commands::Ping { socket } => {
            let socket = socket.unwrap_or_else(default_socket_path);
            println!("Pinging daemon at {:?}", socket);
            // TODO: Connect and ping
        }

        Commands::Run {
            code,
            timeout,
            memory,
            network,
        } => {
            println!("Running directly (no daemon)");
            println!("Code: {}", code);
            println!(
                "Timeout: {}s, Memory: {}MB, Network: {}",
                timeout, memory, network
            );

            // TODO: Use leeward_core directly to execute
            let _config = leeward_core::SandboxConfig::builder()
                .timeout_secs(timeout)
                .memory_limit_mb(memory)
                .allow_network(network)
                .build();
        }
    }

    Ok(())
}
