use crate::config::Config;
use crate::utils::{error_msg, info_msg, success_msg};
use dialoguer::{Select, Confirm, Input};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::core::install::{
    InstallMessage,
    resolve_source,
    extract_and_scan,
    finalize_installation,
    find_desktop_files_with_target,
};

pub async fn install_app(
    config: &Config,
    source: &str,
    app_name_opt: Option<&str>,
    use_root_opt: Option<&str>,
    category_opt: Option<&str>,
    is_cli: bool,
    tx: Option<tokio::sync::mpsc::UnboundedSender<InstallMessage>>,
) -> Result<(), crate::error::KoreError> {
    let mut actual_tarball = PathBuf::from(source);
    let mut downloaded = false;
    let mut repo_name_opt: Option<String> = None;
    let mut repo_package_name_opt: Option<String> = None;
    let mut repo_category_opt: Option<String> = None;
    let mut repo_requires_root_opt: Option<bool> = None;
    let mut repo_terminal_opt: Option<bool> = None;

    if !actual_tarball.exists() {
        if let Some(resolved) = resolve_source(config, source).await? {
            repo_name_opt = resolved.repo_name;
            repo_package_name_opt = resolved.repo_package_name;
            repo_category_opt = resolved.repo_category;
            repo_requires_root_opt = resolved.repo_requires_root;
            repo_terminal_opt = resolved.repo_terminal;
            
            let url = &resolved.url;
            if resolved.is_git {
                if is_cli { info_msg(&format!("Fetching releases for {}...", repo_name_opt.as_deref().unwrap_or("repository"))); }
                        match crate::core::download::get_latest_release_assets(url).await {
                            Ok(assets) => {
                                if assets.is_empty() {
                                    if is_cli { error_msg("No suitable tarball assets found in the latest release."); }
                                } else {
                                    let selected_asset_idx = if assets.len() == 1 {
                                        0
                                    } else {
                                        if !is_cli {
                                            if let Some(t) = &tx {
                                                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                                                let names: Vec<String> = assets.iter().map(|a| a.name.clone()).collect();
                                                let _ = t.send(InstallMessage::SelectAsset(names, reply_tx));
                                                match reply_rx.await {
                                                    Ok(idx) => idx,
                                                    Err(_) => 0,
                                                }
                                            } else {
                                                0
                                            }
                                        } else {
                                            let choices: Vec<String> = assets.iter().map(|a| a.name.clone()).collect();
                                            info_msg("Multiple tarballs found. Please select one:");
                                            Select::new().with_prompt("Tarball").items(&choices).default(0).interact().unwrap_or(0)
                                        }
                                    };
                                    let selected_asset = &assets[selected_asset_idx];
                                    
                                    let tmp_dir = std::env::temp_dir().join("tm_downloads");
                                    std::fs::create_dir_all(&tmp_dir)?;
                                    
                                    if is_cli { info_msg(&format!("Downloading {}...", selected_asset.name)); }
                                    match crate::core::download::download_file(&selected_asset.browser_download_url, &tmp_dir, tx.clone()).await {
                                        Ok(path) => {
                                            actual_tarball = path;
                                            downloaded = true;
                                            if is_cli { info_msg("Download complete!"); }
                                        }
                                        Err(e) => {
                                            if is_cli { error_msg(&format!("Failed to download: {}", e)); }
                                            if let Some(t) = &tx { let _ = t.send(InstallMessage::Progress(format!("Error: {}", e), -1.0)); }
                                            return Err(crate::error::KoreError::Generic(e.to_string()));
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                if is_cli { error_msg(&format!("Failed to query release API: {}", e)); }
                                if let Some(t) = &tx { let _ = t.send(InstallMessage::Progress(format!("Error: {}", e), -1.0)); }
                                return Err(crate::error::KoreError::Generic(e.to_string()));
                            }
                        }
            } else {
                if is_cli { info_msg(&format!("{} is not a known Git provider. Treating as direct download link...", url)); }
                
                let resolved_url = match crate::core::dynamic_links::resolve_dynamic_url(url).await {
                    Ok(u) => u,
                    Err(e) => {
                        if is_cli { error_msg(&format!("Failed to resolve dynamic URL: {}", e)); }
                        return Err(crate::error::KoreError::Generic(e.to_string()));
                    }
                };

                let tmp_dir = std::env::temp_dir().join("tm_downloads");
                std::fs::create_dir_all(&tmp_dir)?;
                
                if is_cli { info_msg(&format!("Downloading from {}...", resolved_url)); }
                match crate::core::download::download_file(&resolved_url, &tmp_dir, tx.clone()).await {
                    Ok(path) => {
                        actual_tarball = path;
                        downloaded = true;
                        if is_cli { info_msg("Download complete!"); }
                    }
                    Err(e) => {
                        if is_cli { error_msg(&format!("Failed to download: {}", e)); }
                        if let Some(t) = &tx { let _ = t.send(InstallMessage::Progress(format!("Error: {}", e), -1.0)); }
                        return Err(crate::error::KoreError::Generic(e.to_string()));
                    }
                }
            }
        } else {
            if is_cli { error_msg(&format!("The file '{}' does not exist, and no repository matches this name.", source)); }
            if let Some(t) = &tx { let _ = t.send(InstallMessage::Progress("File or repository not found".to_string(), -1.0)); }
            return Err(crate::error::KoreError::Generic("Not found".into()));
        }
    }

    if actual_tarball.exists() {
        if let Some(t) = &tx {
            let _ = t.send(InstallMessage::Progress("Extracting archive... Please wait".to_string(), 50.0));
        }
        let config_clone = config.clone();
        let tarball_clone = actual_tarball.clone();
        let repo_package_clone = repo_package_name_opt.clone();
        let is_cli_clone = is_cli;
        let extract_result = tokio::task::spawn_blocking(move || {
            extract_and_scan(&config_clone, &tarball_clone, repo_package_clone.as_deref(), !is_cli_clone)
        }).await.map_err(|e| crate::error::KoreError::Generic(e.to_string()))??;

        if let Some((target, raw_name_folder, executables, desktop_files)) = extract_result {
            let exec_path = if !is_cli {
                if executables.is_empty() {
                    None
                } else if let Some(t) = &tx {
                    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                    let mut choices: Vec<String> = executables.iter().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).collect();
                    choices.push("Skip / Manual link later".to_string());
                    let _ = t.send(InstallMessage::SelectBinary(choices, reply_tx));
                    match reply_rx.await {
                        Ok(idx) if idx < executables.len() => Some(executables[idx].clone()),
                        _ => None,
                    }
                } else {
                    Some(executables[0].clone())
                }
            } else {
                info_msg("Select the main executable binary:");
                let mut choices: Vec<String> = executables.iter().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).collect();
                choices.push("Enter path manually".to_string());
                choices.push("Skip / Manual link later".to_string());
                
                let sel = Select::new()
                    .with_prompt("Binary")
                    .items(&choices)
                    .default(0)
                    .interact()
                    .unwrap_or(choices.len() - 1);
                
                if sel < executables.len() {
                    Some(executables[sel].clone())
                } else if sel == executables.len() {
                    // Manual input
                    let path_str: String = Input::new()
                        .with_prompt("Enter the relative path to the binary (e.g. bin/myapp)")
                        .interact_text()
                        .unwrap_or_default();
                    
                    if path_str.is_empty() {
                        None
                    } else {
                        let full_path = target.join(path_str);
                        if full_path.exists() {
                            Some(full_path)
                        } else {
                            error_msg("The specified path does not exist.");
                            None
                        }
                    }
                } else {
                    None
                }
            };

            let bundled_desktop = if desktop_files.is_empty() {
                None
            } else {
                if !is_cli {
                    if let Some(t) = &tx {
                        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                        let mut choices: Vec<String> = desktop_files.iter().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).collect();
                        choices.push("Skip / Generate New".to_string());
                        let _ = t.send(InstallMessage::SelectDesktop(choices, reply_tx));
                        match reply_rx.await {
                            Ok(idx) if idx < desktop_files.len() => Some(desktop_files[idx].clone()),
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    info_msg("Desktop file(s) found in the archive. Do you want to use one?");
                    let mut choices: Vec<String> = desktop_files.iter().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).collect();
                    choices.push("Skip / Generate New".to_string());
                    let sel = Select::new().with_prompt("Desktop File").items(&choices).default(0).interact().unwrap_or(choices.len() - 1);
                    if sel < desktop_files.len() {
                        Some(desktop_files[sel].clone())
                    } else {
                        None
                    }
                }
            };

            if let Some(exec_path) = exec_path {
                let app_name = app_name_opt
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        if let Some(repo_name) = &repo_name_opt {
                            repo_name.clone()
                        } else if downloaded {
                            source.to_string()
                        } else {
                            raw_name_folder.clone()
                        }
                    });

                let use_root = use_root_opt
                    .map(|s| s.to_lowercase() == "si" || s.to_lowercase() == "yes" || s.to_lowercase() == "s")
                    .unwrap_or_else(|| repo_requires_root_opt.unwrap_or(false));

                let mut use_terminal = repo_terminal_opt.unwrap_or(false);
                if use_root {
                    use_terminal = false;
                }

                let category = category_opt
                    .map(|s| s.to_string())
                    .or(repo_category_opt)
                    .unwrap_or_else(|| "Utility".to_string());
                
                if let Some(t) = &tx {
                    let _ = t.send(InstallMessage::Progress("Finalizing installation...".to_string(), 90.0));
                }
                let config_clone = config.clone();
                let target_clone = target.clone();
                let exec_path_clone = exec_path.clone();
                let app_name_clone = app_name.clone();
                let use_root_clone = use_root;
                let use_terminal_clone = use_terminal;
                let category_clone = category.clone();
                let bundled_desktop_clone = bundled_desktop.clone();
                let is_cli_clone = is_cli;

                tokio::task::spawn_blocking(move || {
                    finalize_installation(&config_clone, &target_clone, &exec_path_clone, &app_name_clone, use_root_clone, use_terminal_clone, &category_clone, bundled_desktop_clone, !is_cli_clone)
                }).await.map_err(|e| crate::error::KoreError::Generic(e.to_string()))??;
                if let Some(t) = &tx {
                    let _ = t.send(InstallMessage::Progress(format!("Successfully installed {}", app_name), 100.0));
                }
                if !is_cli {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            } else {
                if let Some(t) = &tx {
                    let _ = t.send(InstallMessage::Progress("Installation cancelled: No binary selected".to_string(), -1.0));
                }
            }
        } else {
            if let Some(t) = &tx {
                let _ = t.send(InstallMessage::Progress("Failed to extract the archive".to_string(), -1.0));
            }
        }
    } else {
        if let Some(t) = &tx {
            let _ = t.send(InstallMessage::Progress("Archive file not found after download".to_string(), -1.0));
        }
    }
    
    // Clean up downloaded file (ensure it happens)
    if downloaded && actual_tarball.exists() {
        let _ = std::fs::remove_file(&actual_tarball);
    }
    
    Ok(())
}

pub fn remove_app(config: &Config, app_name: &str, is_cli: bool, silent: bool) -> Result<(), crate::error::KoreError> {
    let mut target_path = config.install_dir.join(app_name);

    if !target_path.exists() {
        // Suggest name if it doesn't exist exactly (case insensitive or similar)
        let entries = fs::read_dir(&config.install_dir)?;
        let mut found = None;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.contains(&app_name.to_lowercase()) {
                found = Some(entry.file_name());
                break;
            }
        }

        if let Some(suggestion) = found {
            target_path = config.install_dir.join(&suggestion);
        } else {
            if !silent { error_msg(&format!("Nothing related to '{}' was found in {}", app_name, config.install_dir.display())); }
            if !is_cli {
                if !silent { println!("\nPress Enter to return..."); }
                let _ = Input::<String>::new().allow_empty(true).with_prompt("").interact_text();
            }
            return Ok(());
        }
    }

    let confirm = if is_cli { 
        true 
    } else {
        Confirm::new()
            .with_prompt(format!("Completely delete '{}'?", target_path.file_name().unwrap_or_default().to_string_lossy()))
            .default(false)
            .interact()
            .unwrap_or(false)
    };

    if !confirm {
        if !silent { info_msg("Operation cancelled."); }
        return Ok(());
    }

    if !silent { info_msg("Searching for associated files..."); }
    // Locate and iterate through .desktop files containing this path
    let target_str = target_path.to_string_lossy().to_string();
    for path in find_desktop_files_with_target(config, &target_str) {
        let app_name_from_file = path.file_stem().unwrap_or_default();
        let associated_bin = config.bin_dir.join(app_name_from_file);
        let _ = fs::remove_file(&associated_bin);
        let _ = fs::remove_file(&path);
        if !silent { success_msg(&format!("Removed shortcut: {}", path.file_name().unwrap_or_default().to_string_lossy())); }
    }

    let _ = fs::remove_dir_all(&target_path);
    // Additionally remove a homonymous binary directly in case of broken links
    let _ = fs::remove_file(config.bin_dir.join(target_path.file_name().unwrap_or_default()));

    if let Ok(mut child) = Command::new("update-desktop-database")
        .arg(&config.apps_dir)
        .spawn() 
    {
        let _ = child.wait();
    }
    let _ = std::process::Command::new("touch").arg(&config.apps_dir).status();

    if !silent { success_msg(&format!("{} successfully removed!", target_path.file_name().unwrap_or_default().to_string_lossy())); }
    if !is_cli {
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    
    Ok(())
}
