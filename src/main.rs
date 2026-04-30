mod cli;
mod tui;

use kpm::config;
use kpm::core;
use kpm::repo;
use kpm::utils;

use clap::Parser;
use cli::{Cli, Commands};
use config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::new();
    config.setup_dirs()?;

    let cli = Cli::parse();
    
    let is_cli_mode = cli.command.is_some() || cli.update_bin;
    utils::IS_CLI.store(is_cli_mode, std::sync::atomic::Ordering::Relaxed);
    
    let _guard = config.setup_logging()?;

    if cli.update_bin {
        core::update_kpm(&config).await?;
        return Ok(());
    }

    let mut had_failures = false;

    match &cli.command {
        Some(Commands::List) => {
            core::list_cli(&config);
        }
        Some(Commands::Remove { app_names }) => {
            for name in app_names {
                if let Err(e) = core::remove_app(&config, name, true, false) {
                    utils::error_msg(&format!("Failed to remove {}: {}", name, e));
                    had_failures = true;
                }
            }
        }
        Some(Commands::Install { sources, app_name, use_root, category }) => {
            let multi = sources.len() > 1;
            for source in sources {
                let res = if multi {
                    core::install_app(&config, source, None, None, None, true, None, false).await
                } else {
                    core::install_app(&config, source, app_name.as_deref(), use_root.as_deref(), category.as_deref(), true, None, false).await
                };

                if let Err(e) = res {
                    utils::error_msg(&format!("Failed to install {}: {}", source, e));
                    had_failures = true;
                }
            }
        }
        Some(Commands::Repo { repo_command }) => {
            match repo_command {
                cli::RepoCommands::List => {
                    let off_count = repo::get_official_repos(&config).len();
                    let com_count = repo::get_community_repos(&config).len();
                    let usr_count = repo::get_user_repos(&config).len();

                    println!("\x1b[1;36mRepositories:\x1b[0m");
                    println!("  - Official Repository: {} packages", off_count);
                    println!("  - Community Repositories: {} packages", com_count);
                    println!("  - Custom Repositories: {} packages", usr_count);
                }
                cli::RepoCommands::PkgList => {
                    let all = repo::get_all_repos(&config);
                    if all.is_empty() {
                        utils::info_msg("No packages found.");
                    } else {
                        let mut official: Vec<_> = all.iter().filter(|r| r.repo_type == repo::RepoType::Official).collect();
                        let mut community: Vec<_> = all.iter().filter(|r| r.repo_type == repo::RepoType::Community).collect();
                        let mut custom: Vec<_> = all.iter().filter(|r| r.repo_type == repo::RepoType::User).collect();

                        let sort_fn = |a: &&repo::RepoSource, b: &&repo::RepoSource| {
                            let name_a = if a.repo.package_name.is_empty() { &a.repo.name } else { &a.repo.package_name };
                            let name_b = if b.repo.package_name.is_empty() { &b.repo.name } else { &b.repo.package_name };
                            name_a.to_lowercase().cmp(&name_b.to_lowercase())
                        };

                        official.sort_by(sort_fn);
                        community.sort_by(sort_fn);
                        custom.sort_by(sort_fn);

                        if !official.is_empty() {
                            println!("\x1b[1;36mOfficial Repositories:\x1b[0m");
                            for r in official {
                                let display_name = if r.repo.package_name.is_empty() { &r.repo.name } else { &r.repo.package_name };
                                println!("  - {}", display_name);
                            }
                            println!();
                        }

                        if !community.is_empty() {
                            println!("\x1b[1;36mCommunity Repositories:\x1b[0m");
                            for r in community {
                                let display_name = if r.repo.package_name.is_empty() { &r.repo.name } else { &r.repo.package_name };
                                println!("  - {}", display_name);
                            }
                            println!();
                        }

                        if !custom.is_empty() {
                            println!("\x1b[1;36mCustom Repositories:\x1b[0m");
                            for r in custom {
                                let display_name = if r.repo.package_name.is_empty() { &r.repo.name } else { &r.repo.package_name };
                                println!("  - {}", display_name);
                            }
                        }
                    }
                }
                cli::RepoCommands::PkgSearch { query } => {
                    let all = repo::get_all_repos(&config);
                    let q = query.to_lowercase();
                    let matches: Vec<_> = all.into_iter().filter(|r| r.repo.name.to_lowercase().contains(&q) || r.repo.package_name.to_lowercase().contains(&q)).collect();
                    if matches.is_empty() {
                        utils::info_msg(&format!("No packages found matching '{}'.", query));
                    } else {
                        println!("\x1b[1;36mSearch Results:\x1b[0m");
                        for r in matches {
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
                    match repo::sync_repos(&config).await {
                        Ok(_) => utils::success_msg("Repositories successfully synced!"),
                        Err(e) => utils::error_msg(&format!("Failed to sync repositories: {}", e)),
                    }
                }
                cli::RepoCommands::Add { name, package_name, url, category, requires_root } => {
                    match repo::add_user_repo(&config, name, package_name, url, category, *requires_root).await {
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
                if let Some(repo_source) = all_repos.iter().find(|r| r.repo.name.to_lowercase() == target.to_lowercase() || (!r.repo.package_name.is_empty() && r.repo.package_name.to_lowercase() == target.to_lowercase())) {
                    if let Err(e) = core::install_app(&config, &repo_source.repo.name, Some(&repo_source.repo.name), None, None, true, None, true).await {
                        utils::error_msg(&format!("Failed to update {}: {}", repo_source.repo.name, e));
                        had_failures = true;
                    }
                } else {
                    utils::error_msg(&format!("Application '{}' does not belong to any repository.", target));
                    had_failures = true;
                }
            } else {
                let mut updated_any = false;
                if let Ok(entries) = std::fs::read_dir(&config.install_dir) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir() {
                            let app = entry.file_name().to_string_lossy().to_string();
                            if let Some(repo_source) = all_repos.iter().find(|r| r.repo.name.to_lowercase() == app.to_lowercase() || (!r.repo.package_name.is_empty() && r.repo.package_name.to_lowercase() == app.to_lowercase())) {
                                utils::info_msg(&format!("Updating {}...", repo_source.repo.name));
                                match core::install_app(&config, &repo_source.repo.name, Some(&repo_source.repo.name), None, None, true, None, true).await {
                                    Ok(_) => updated_any = true,
                                    Err(e) => {
                                        utils::error_msg(&format!("Failed to update {}: {}", repo_source.repo.name, e));
                                        had_failures = true;
                                    }
                                }
                            }
                        }
                    }
                } else {
                    had_failures = true;
                    utils::error_msg("Could not read installation directory for update scan.");
                }
                if !updated_any {
                    utils::info_msg("No installed applications matched any repository for updating.");
                } else {
                    utils::success_msg("All applicable repositories have been processed.");
                }
            }
        }
        None => {
            tui::main_menu(&config).await?;
        }
    }

    if had_failures {
        return Err(anyhow::anyhow!("One or more operations failed."));
    }

    Ok(())
}
