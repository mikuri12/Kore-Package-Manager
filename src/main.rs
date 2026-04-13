mod cli;
mod config;
mod core;
mod tui;
mod utils;

use clap::Parser;
use cli::{Cli, Commands};
use config::Config;

fn main() {
    let config = Config::new();
    if let Err(e) = config.setup_dirs() {
        utils::error_msg(&format!("Error setting up directories: {}", e));
        return;
    }

    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::List) => {
            core::list_cli(&config);
        }
        Some(Commands::Remove { app_name }) => {
            let _ = core::remove_app(&config, app_name, true, false);
        }
        Some(Commands::Install { tarball, app_name, use_root, category }) => {
            let app = if app_name.is_empty() { None } else { Some(app_name.as_str()) };
            let _ = core::install_app(&config, tarball, app, Some(use_root), Some(category), true);
        }
        None => {
            // Si no hay argumentos, abrimos la interfaz TUI (Terminal User Interface)
            let _ = tui::main_menu(&config);
        }
    }
}
