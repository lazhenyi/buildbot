//! Buildbot Dispatcher - GitHub Actions-style CI system in Rust
//!
//! This is the main entry point for the Dispatcher CI system.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use buildbot::master::MasterService;
use buildbot::config::ConfigLoader;

#[derive(Parser)]
#[command(name = "buildbot")]
#[command(about = "Buildbot Dispatcher - GitHub Actions-style CI system", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Base directory for buildbot data
    #[arg(short, long, default_value = ".")]
    basedir: PathBuf,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the dispatcher master
    Master {
        /// Configuration file path (YAML)
        #[arg(short, long, default_value = "master.cfg")]
        config: PathBuf,

        /// API port (overrides config)
        #[arg(short, long)]
        api_port: Option<u16>,

        /// Web port (overrides config)
        #[arg(short, long)]
        web_port: Option<u16>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let env_filter = if cli.verbose {
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug"))
    } else {
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Buildbot Dispatcher starting...");

    match cli.command {
        Commands::Master {
            config,
            api_port,
            web_port,
        } => {
            tracing::info!("Starting Buildbot Dispatcher Master");
            tracing::info!("Config: {:?}", config);
            tracing::info!("Basedir: {:?}", cli.basedir);

            run_master(cli.basedir, config, api_port, web_port).await?;
        }
    }

    Ok(())
}

async fn run_master(
    basedir: PathBuf,
    config_path: PathBuf,
    cli_api_port: Option<u16>,
    cli_web_port: Option<u16>,
) -> anyhow::Result<()> {
    use buildbot::db::Database;

    // Load configuration from YAML file
    let mut loader = ConfigLoader::new();
    loader.set_base_dir(basedir.clone());

    let yaml_config = loader.load_from_file(&config_path).await?;
    tracing::info!("Loaded configuration from '{}'", config_path.display());

    let mut master_config = yaml_config.into_master_config(basedir.clone());

    // CLI arguments override config file
    if let Some(port) = cli_api_port {
        master_config.api_port = port;
    }
    if let Some(port) = cli_web_port {
        master_config.web_port = port;
    }

    tracing::info!("Master name: {}", master_config.name);
    tracing::info!("API port: {}, Web port: {}", master_config.api_port, master_config.web_port);

    // Initialize database
    let db = Database::new(&master_config.database_url).await?;

    // Create and start master service
    let mut service = MasterService::new(master_config, db);
    service.start().await?;

    // Keep running until shutdown
    service.wait_for_shutdown().await;

    service.stop().await?;
    tracing::info!("Master stopped");
    Ok(())
}
