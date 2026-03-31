use clap::Parser;
use mbr_core::storage::config::Config;
use std::path::PathBuf;

mod cli;

use cli::dispatcher::Dispatcher;
use cli::main_types::Cli;
use cli::output::{exit_code_for, print_json_error};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let json_mode = cli.json;
    let verbose = cli.verbose;
    let use_colors = cli.color.should_use_colors();

    let config_path = cli
        .config_dir
        .as_ref()
        .map(|dir| PathBuf::from(dir).join("config.toml"));

    let config = match Config::load(config_path) {
        Ok(config) => config,
        Err(err) => {
            let app_err = mbr_core::error::AppError::from(err);
            if json_mode {
                print_json_error(&app_err);
            } else {
                eprintln!("Error loading config: {}", app_err);
            }
            std::process::exit(exit_code_for(&app_err));
        }
    };

    if verbose {
        eprintln!("Verbose mode is enabled");
        if let Some(config_dir) = &cli.config_dir {
            eprintln!("Using config directory: {}", config_dir);
        }
    }

    let config_dir = cli.config_dir.as_deref();
    let dispatcher = Dispatcher::new(config, verbose, use_colors, json_mode);

    if let Err(e) = dispatcher.dispatch(cli.command, config_dir).await {
        if json_mode {
            print_json_error(&e);
        } else {
            eprintln!("Error: {}", e);
        }
        std::process::exit(exit_code_for(&e));
    }
}
