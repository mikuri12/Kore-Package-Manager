mod cli;
mod config;
mod core;
pub mod download;
pub mod repo;
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
        Some(Commands::Install { source, app_name, use_root, category }) => {
            let app = if app_name.is_empty() { None } else { Some(app_name.as_str()) };
            core::install_app(&config, source, app, Some(use_root), Some(category), true)?;
        }
        Some(Commands::Repo { repo_command }) => {
            match repo_command {
                cli::RepoCommands::List => {
                    let all = repo::get_all_repos(&config);
                    if all.is_empty() {
                        utils::info_msg("No repositories found.");
                    } else {
                        println!("\x1b[1;36mRepositories:\x1b[0m");
                        for r in all {
                            let r_type = match r.repo_type {
                                repo::RepoType::Official => "Official",
                                repo::RepoType::Community => "Community",
                                repo::RepoType::User => "Custom",
                            };
                            println!("  - [{}] {} ({}) - {} ({})", r_type, r.repo.name, r.repo.package_name, r.repo.url, r.repo.category);
                        }
                    }
                }
                cli::RepoCommands::Sync => {
                    utils::info_msg("Syncing repositories from GitHub...");
                    match repo::sync_repos(&config) {
                        Ok(_) => utils::success_msg("Repositories successfully synced!"),
                        Err(e) => utils::error_msg(&format!("Failed to sync repositories: {}", e)),
                    }
                }
                cli::RepoCommands::Add { name, package_name, url, category, requires_root } => {
                    match repo::add_user_repo(&config, name, package_name, url, category, *requires_root) {
                        Ok(_) => utils::success_msg(&format!("Repository '{}' added.", name)),
                        Err(e) => utils::error_msg(&format!("Failed to add repository: {}", e)),
                    }
                }
                cli::RepoCommands::Remove { name } => {
                    match repo::remove_user_repo(&config, name) {
                        Ok(true) => utils::success_msg(&format!("Repository '{}' removed.", name)),
                        Ok(false) => utils::error_msg(&format!("Repository '{}' not found.", name)),
                        Err(e) => utils::error_msg(&format!("Failed to remove repository: {}", e)),
                    }
                }
            }
        }
        Some(Commands::Update { app_name }) => {
            let all_repos = repo::get_all_repos(&config);
            if let Some(target) = app_name {
                if let Some(repo_source) = all_repos.iter().find(|r| r.repo.name.to_lowercase() == target.to_lowercase()) {
                    let _ = core::install_app(&config, &repo_source.repo.name, Some(&repo_source.repo.name), None, None, true);
                } else {
                    utils::error_msg(&format!("Application '{}' does not belong to any repository.", target));
                }
            } else {
                let mut updated_any = false;
                if let Ok(entries) = std::fs::read_dir(&config.apps_dir) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir() {
                            let app = entry.file_name().to_string_lossy().to_string();
                            if let Some(repo_source) = all_repos.iter().find(|r| r.repo.name.to_lowercase() == app.to_lowercase()) {
                                utils::info_msg(&format!("Updating {}...", repo_source.repo.name));
                                let _ = core::install_app(&config, &repo_source.repo.name, Some(&repo_source.repo.name), None, None, true);
                                updated_any = true;
                            }
                        }
                    }
                }
                if !updated_any {
                    utils::info_msg("No installed applications matched any repository for updating.");
                } else {
                    utils::success_msg("All applicable repositories have been processed.");
                }
            }
        }
        None => {
            // If there are no arguments, open the TUI (Terminal User Interface)
            tui::main_menu(&config)?;
        }
    }
    
    Ok(())
}
