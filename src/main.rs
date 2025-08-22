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

    let mut config = match Config::load(config_path.clone()) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Error loading config: {}", err);
            std::process::exit(1);
        }
    };

    // Determine the profile to use
    let profile_name = cli
        .profile
        .or(config.default_profile.clone())
        .unwrap_or_else(|| "default".to_string());

    // Create a default profile if it doesn't exist
    if config.get_profile(&profile_name).is_none() {
        if cli.verbose {
            println!("Creating default profile: {}", profile_name);
        }

        use mbr_cli::storage::config::Profile;
        let default_profile = Profile {
            metabase_url: "http://localhost:3000".to_string(),
            email: None,
        };

        config.set_profile(profile_name.clone(), default_profile);

        // Set as default if no default is set
        if config.default_profile.is_none() {
            config.default_profile = Some(profile_name.clone());
        }

        // Save the updated config
        if let Err(err) = config.save(config_path) {
            if cli.verbose {
                println!("Warning: Failed to save config: {}", err);
            }
        }
    }

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

    // Create dispatcher
    let dispatcher = Dispatcher::new(config, credentials, cli.verbose, cli.api_key);

    // Execute the command
    if let Err(e) = dispatcher.dispatch(cli.command).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
