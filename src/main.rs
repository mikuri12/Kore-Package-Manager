mod cli;
mod config;
mod core;
mod tui;
mod utils;

use clap::Parser;
use cli::{Cli, Commands};
use config::Config;

fn main() -> anyhow::Result<()> {
    let config = Config::new();
    config.setup_dirs()?;

    let cli = Cli::parse();
    
    let is_cli_mode = cli.command.is_some() || cli.update_bin;
    crate::utils::IS_CLI.store(is_cli_mode, std::sync::atomic::Ordering::Relaxed);
    
    let _guard = config.setup_logging()?;

    if cli.update_bin {
        core::update_tm(&config)?;
        return Ok(());
    }

    match &cli.command {
        Some(Commands::List) => {
            core::list_cli(&config);
        }
        Some(Commands::Remove { app_name }) => {
            core::remove_app(&config, app_name, true, false)?;
        }
        Some(Commands::Install { tarball, app_name, use_root, category }) => {
            let app = if app_name.is_empty() { None } else { Some(app_name.as_str()) };
            core::install_app(&config, tarball, app, Some(use_root), Some(category), true)?;
        }
        None => {
            // If there are no arguments, open the TUI (Terminal User Interface)
            tui::main_menu(&config)?;
        }
    }
    
    Ok(())
}
