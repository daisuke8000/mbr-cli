use clap::Parser;
use rmb_cli::cli::main_types::Cli;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.verbose {
        println!("Verbose mode is enabled");

        if let Some(profile) = &cli.profile {
            println!("Using profile: {}", profile);
        }

        if let Some(config_dir) = &cli.config_dir {
            println!("Using config directory: {}", config_dir);
        }

        if cli.api_key.is_some() {
            println!("Using API key Provided via env or command line");
        }
    }

    match &cli.command {
        rmb_cli::cli::main_types::Commands::Auth { command } => {
            println!("Auth command: {:?}", command);
        }
        rmb_cli::cli::main_types::Commands::Config { command } => {
            println!("Config command: {:?}", command);
        }
        rmb_cli::cli::main_types::Commands::Question { command } => {
            println!("Question command: {:?}", command);
        }
    }

    Ok(())
}
