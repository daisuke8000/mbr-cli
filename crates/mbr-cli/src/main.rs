use clap::Parser;
use mbr_core::storage::config::Config;
use std::path::PathBuf;

mod cli;

use cli::dispatcher::Dispatcher;
use cli::main_types::Cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Load Config
    let config_path = cli
        .config_dir
        .as_ref()
        .map(|dir| PathBuf::from(dir).join("config.toml"));

    let config = match Config::load(config_path.clone()) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Error loading config: {}", err);
            std::process::exit(1);
        }
    };

    if cli.verbose {
        println!("Verbose mode is enabled");

        if let Some(config_dir) = &cli.config_dir {
            println!("Using config directory: {}", config_dir);
        }

        if cli.api_key.as_ref().is_some_and(|key| !key.is_empty()) {
            println!("Using API key provided via env or command line");
        }
    }

    // Create dispatcher
    let dispatcher = Dispatcher::new(config, cli.verbose, cli.api_key);

    // Execute the command
    if let Err(e) = dispatcher.dispatch(cli.command).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
