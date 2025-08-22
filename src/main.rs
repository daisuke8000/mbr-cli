use clap::Parser;
use mbr_cli::cli::dispatcher::Dispatcher;
use mbr_cli::cli::main_types::Cli;
use mbr_cli::storage::config::Config;
use mbr_cli::storage::credentials::Credentials;
use std::path::PathBuf;

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

    // Determine the profile to use (default to "default" if not specified)
    let profile_name = cli.profile.unwrap_or_else(|| "default".to_string());

    // Profile creation and config file saving is now handled by Dispatcher::new()

    if cli.verbose {
        println!("Verbose mode is enabled");
        println!("Using profile: {}", profile_name);

        if let Some(config_dir) = &cli.config_dir {
            println!("Using config directory: {}", config_dir);
        }

        if cli.api_key.as_ref().is_some_and(|key| !key.is_empty()) {
            println!("Using API key Provided via env or command line");
        }
    }

    // Load Credentials
    let credentials = match Credentials::load(&profile_name) {
        Ok(creds) => creds,
        Err(err) => {
            eprintln!("Error loading credentials: {}", err);
            Credentials::new(profile_name.clone())
        }
    };

    // Create dispatcher (handles profile creation and config file saving)
    let dispatcher = Dispatcher::new(config, credentials, cli.verbose, cli.api_key, config_path);

    // Execute the command
    if let Err(e) = dispatcher.dispatch(cli.command).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
