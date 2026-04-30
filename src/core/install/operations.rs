use crate::config::Config;
use crate::utils::{error_msg, info_msg, success_msg};
use dialoguer::{Select, Confirm, Input};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::install::{
    InstallMessage,
    resolve_source,
    extract_and_scan,
    finalize_installation,
    find_desktop_files_with_target,
};

fn create_unique_temp_download_dir() -> Result<PathBuf, crate::error::KoreError> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let pid = std::process::id();
    let tmp_dir = std::env::temp_dir().join(format!("kpm_downloads_{}_{}", pid, ts));
    std::fs::create_dir_all(&tmp_dir)?;
    Ok(tmp_dir)
}

pub async fn install_app(
    config: &Config,
    source: &str,
    app_name_opt: Option<&str>,
    use_root_opt: Option<&str>,
    category_opt: Option<&str>,
    is_cli: bool,
    tx: Option<tokio::sync::mpsc::UnboundedSender<InstallMessage>>,
    update_mode: bool,
) -> Result<(), crate::error::KoreError> {
    tracing::info!(operation = "install", source = source, cli = is_cli, "Install flow started");
    let mut actual_tarball = PathBuf::from(source);
    let mut downloaded = false;
    let mut repo_name_opt: Option<String> = None;
    let mut _repo_package_name_opt: Option<String> = None;
    let mut repo_category_opt: Option<String> = None;
    let mut repo_requires_root_opt: Option<bool> = None;
    let mut repo_terminal_opt: Option<bool> = None;
    let mut repo_version_opt: Option<String> = None;
    let mut saved_asset: Option<String> = None;
    let mut saved_binary: Option<String> = None;
    let mut saved_desktop: Option<String> = None;

    if !actual_tarball.exists() {
        tracing::info!(operation = "install", source = source, step = "resolve_source", "Resolving source");
        if let Some(resolved) = resolve_source(config, source).await? {
            repo_name_opt = resolved.repo_name;
            _repo_package_name_opt = resolved.repo_package_name;
            repo_category_opt = resolved.repo_category;
            repo_requires_root_opt = resolved.repo_requires_root;
            repo_terminal_opt = resolved.repo_terminal;
            
            let url = &resolved.url;
            if resolved.is_git {
                if is_cli { info_msg(&format!("Fetching releases for {}...", repo_name_opt.as_deref().unwrap_or("repository"))); }
                        match crate::core::download::get_latest_release_assets(url).await {
                            Ok((version, assets)) => {
                                repo_version_opt = Some(version.clone());

                                let display_name_for_update = app_name_opt
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| {
                                        if let Some(repo_name) = &repo_name_opt {
                                            repo_name.clone()
                                        } else {
                                            "".to_string()
                                        }
                                    });
                                let prospective_name = display_name_for_update.to_lowercase().replace(' ', "-");

                                let mut local_version_opt: Option<String> = None;
                                if update_mode && !prospective_name.is_empty() {
                                    let manifest_path = config.install_dir.join(&prospective_name).join(".kpm_manifest.json");
                                    if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                                        if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&content) {
                                            saved_asset = manifest.get("asset_name").and_then(|v| v.as_str()).map(|s| s.to_string());
                                            saved_binary = manifest.get("binary_path").and_then(|v| v.as_str()).map(|s| s.to_string());
                                            saved_desktop = manifest.get("desktop_file").and_then(|v| v.as_str()).map(|s| s.to_string());
                                            
                                            if let Some(local_version) = manifest.get("version").and_then(|v| v.as_str()) {
                                                local_version_opt = Some(local_version.to_string());
                                                if local_version.trim_start_matches('v') == version.trim_start_matches('v') {
                                                    if is_cli {
                                                        info_msg(&format!("{} is already up-to-date ({}).", prospective_name, version));
                                                    }
                                                    if let Some(t) = &tx {
                                                        let _ = t.send(InstallMessage::Progress("Already up-to-date.".to_string(), 100.0));
                                                    }
                                                    return Ok(());
                                                }
                                            }
                                        }
                                    }
                                }

                                if assets.is_empty() {
                                    if is_cli { error_msg("No suitable tarball assets found in the latest release."); }
                                } else {
                                    let mut resolved_idx = None;
                                    if let Some(ref a) = saved_asset {
                                        if let Some(idx) = assets.iter().position(|asset| &asset.name == a) {
                                            resolved_idx = Some(idx);
                                        } else if let Some(ref lv) = local_version_opt {
                                            let attempt1 = a.replace(lv, &version);
                                            if let Some(idx) = assets.iter().position(|asset| asset.name == attempt1) {
                                                resolved_idx = Some(idx);
                                            } else {
                                                let attempt2 = a.replace(lv.trim_start_matches('v'), version.trim_start_matches('v'));
                                                if let Some(idx) = assets.iter().position(|asset| asset.name == attempt2) {
                                                    resolved_idx = Some(idx);
                                                }
                                            }
                                        }
                                    }

                                    let selected_asset_idx = if assets.len() == 1 {
                                        0
                                    } else if let Some(idx) = resolved_idx {
                                        idx
                                    } else {
                                        if !is_cli {
                                            if let Some(t) = &tx {
                                                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                                                let names: Vec<String> = assets.iter().map(|a| a.name.clone()).collect();
                                                let _ = t.send(InstallMessage::SelectAsset(names, reply_tx));
                                                reply_rx.await.unwrap_or(usize::MAX)
                                            } else {
                                                0
                                            }
                                        } else {
                                            let choices: Vec<String> = assets.iter().map(|a| a.name.clone()).collect();
                                            info_msg("Multiple tarballs found. Please select one:");
                                            Select::new().with_prompt("Tarball").items(&choices).default(0).interact().unwrap_or(usize::MAX)
                                        }
                                    };
                                    if selected_asset_idx == usize::MAX {
                                        if let Some(t) = &tx { let _ = t.send(InstallMessage::Progress("Cancelled by user".to_string(), -1.0)); }
                                        return Err(crate::error::KoreError::Generic("Installation cancelled by user".to_string()));
                                    }
                                    let selected_asset = &assets[selected_asset_idx];
                                    saved_asset = Some(selected_asset.name.clone());
                                    
                                    let tmp_dir = create_unique_temp_download_dir()?;
                                    
                                    if is_cli { info_msg(&format!("Downloading {}...", selected_asset.name)); }
                                    tracing::info!(operation = "install", source = source, step = "download_release_asset", asset = %selected_asset.name, "Downloading selected release asset");
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

                let tmp_dir = create_unique_temp_download_dir()?;
                repo_version_opt = Some("latest".to_string());
                
                if is_cli { info_msg(&format!("Downloading from {}...", resolved_url)); }
                tracing::info!(operation = "install", source = source, step = "download_direct_url", url = %resolved_url, "Downloading direct URL");
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
        tracing::info!(operation = "install", source = source, step = "extract", archive = %actual_tarball.display(), "Starting archive extraction");
        if let Some(t) = &tx {
            if t.send(InstallMessage::Progress("Extracting archive... Please wait".to_string(), 50.0)).is_err() {
                tracing::warn!(operation = "install", source = source, step = "extract", "Progress receiver dropped");
            }
        }
        let computed_display_name = app_name_opt
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                if let Some(repo_name) = &repo_name_opt {
                    repo_name.clone()
                } else if downloaded {
                    source.to_string()
                } else {
                    actual_tarball.file_name().unwrap_or_default().to_string_lossy()
                        .replace(".tar.gz", "").replace(".tar.xz", "").replace(".AppImage", "")
                        .replace(".appimage", "").replace(".zip", "")
                }
            });
        let safe_name = computed_display_name.to_lowercase().replace(' ', "-");

        let config_clone = config.clone();
        let tarball_clone = actual_tarball.clone();
        let target_folder_name_opt = Some(safe_name.clone());
        let is_cli_clone = is_cli;
        let is_appimage = actual_tarball.to_string_lossy().to_lowercase().ends_with(".appimage");
        let extract_result = if is_appimage {
            tokio::task::spawn_blocking(move || {
                crate::core::install::appimage::process_appimage(&config_clone, &tarball_clone, target_folder_name_opt.as_deref())
            }).await.map_err(|e| crate::error::KoreError::Generic(e.to_string()))??
        } else {
            tokio::task::spawn_blocking(move || {
                extract_and_scan(&config_clone, &tarball_clone, target_folder_name_opt.as_deref(), !is_cli_clone)
            }).await.map_err(|e| crate::error::KoreError::Generic(e.to_string()))??
        };

        if let Some((target, _raw_name_folder, executables, desktop_files)) = extract_result {
            let exec_path = if let Some(ref b) = saved_binary {
                Some(target.join(b))
            } else if !is_cli {
                if executables.is_empty() {
                    None
                } else if let Some(t) = &tx {
                    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                    let mut choices: Vec<String> = executables.iter().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).collect();
                    choices.push("Skip / Manual link later".to_string());
                    let _ = t.send(InstallMessage::SelectBinary(choices, reply_tx));
                    match reply_rx.await {
                        Ok(idx) if idx == usize::MAX => return Err(crate::error::KoreError::Generic("Installation cancelled by user".to_string())),
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

            let bundled_desktop = if let Some(ref d) = saved_desktop {
                let p = target.join(d);
                if p.exists() { Some(p) } else { None }
            } else if desktop_files.is_empty() {
                None
            } else {
                if !is_cli {
                    if let Some(t) = &tx {
                        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                        let mut choices: Vec<String> = desktop_files.iter().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).collect();
                        choices.push("Skip / Generate New".to_string());
                        let _ = t.send(InstallMessage::SelectDesktop(choices, reply_tx));
                        match reply_rx.await {
                            Ok(idx) if idx == usize::MAX => return Err(crate::error::KoreError::Generic("Installation cancelled by user".to_string())),
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
                let internal_name = safe_name.clone();
                let display_name = computed_display_name.clone();

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
                    if t.send(InstallMessage::Progress("Finalizing installation...".to_string(), 90.0)).is_err() {
                        tracing::warn!(operation = "install", source = source, step = "finalize", "Progress receiver dropped");
                    }
                }
                let config_clone = config.clone();
                let target_clone = target.clone();
                let exec_path_clone = exec_path.clone();
                let internal_name_clone = internal_name.clone();
                let display_name_clone = display_name.clone();
                let use_root_clone = use_root;
                let use_terminal_clone = use_terminal;
                let category_clone = category.clone();
                let bundled_desktop_clone = bundled_desktop.clone();
                let version_clone = repo_version_opt.clone();
                let asset_name_clone = saved_asset.clone();
                let is_cli_clone = is_cli;

                tokio::task::spawn_blocking(move || {
                    finalize_installation(&config_clone, &target_clone, &exec_path_clone, &internal_name_clone, &display_name_clone, use_root_clone, use_terminal_clone, &category_clone, bundled_desktop_clone, version_clone, asset_name_clone, !is_cli_clone)
                }).await.map_err(|e| crate::error::KoreError::Generic(e.to_string()))??;
                tracing::info!(operation = "install", app = %internal_name, step = "finalize", "Installation finalized");
                if let Some(t) = &tx {
                    if t.send(InstallMessage::Progress(format!("Successfully installed {}", display_name), 100.0)).is_err() {
                        tracing::warn!(operation = "install", app = %internal_name, step = "finalize", "Progress receiver dropped");
                    }
                }
                if !is_cli {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            } else {
                if let Some(t) = &tx {
                    if t.send(InstallMessage::Progress("Installation cancelled: No binary selected".to_string(), -1.0)).is_err() {
                        tracing::warn!(operation = "install", source = source, step = "binary_selection", "Progress receiver dropped");
                    }
                }
            }
        } else {
            if let Some(t) = &tx {
                if t.send(InstallMessage::Progress("Failed to extract the archive".to_string(), -1.0)).is_err() {
                    tracing::warn!(operation = "install", source = source, step = "extract", "Progress receiver dropped");
                }
            }
        }
    } else {
        if let Some(t) = &tx {
            if t.send(InstallMessage::Progress("Archive file not found after download".to_string(), -1.0)).is_err() {
                tracing::warn!(operation = "install", source = source, step = "download", "Progress receiver dropped");
            }
        }
    }
    
    if downloaded {
        if actual_tarball.exists() {
            let _ = std::fs::remove_file(&actual_tarball);
        }
        if let Some(parent) = actual_tarball.parent() {
            if parent
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("kpm_downloads_"))
                .unwrap_or(false)
            {
                let _ = std::fs::remove_dir_all(parent);
            }
        }
    }
    
    tracing::info!(operation = "install", source = source, "Install flow finished");
    Ok(())
}

pub fn remove_app(config: &Config, app_name: &str, is_cli: bool, silent: bool) -> Result<(), crate::error::KoreError> {
    tracing::info!(operation = "remove", app = app_name, cli = is_cli, "Remove flow started");
    let mut target_path = config.install_dir.join(app_name);

    if !target_path.exists() {
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
    let mut critical_errors: Vec<String> = Vec::new();

    let target_str = target_path.to_string_lossy().to_string();
    for path in find_desktop_files_with_target(config, &target_str) {
        let app_name_from_file = path.file_stem().unwrap_or_default();
        let associated_bin = config.bin_dir.join(app_name_from_file);

        match fs::remove_file(&associated_bin) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => critical_errors.push(format!("Failed to remove binary '{}': {}", associated_bin.display(), e)),
        }

        match fs::remove_file(&path) {
            Ok(_) => {
                if !silent { success_msg(&format!("Removed shortcut: {}", path.file_name().unwrap_or_default().to_string_lossy())); }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => critical_errors.push(format!("Failed to remove shortcut '{}': {}", path.display(), e)),
        }
    }

    match fs::remove_dir_all(&target_path) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => critical_errors.push(format!("Failed to remove directory '{}': {}", target_path.display(), e)),
    }

    let homonymous_bin = config.bin_dir.join(target_path.file_name().unwrap_or_default());
    if let Err(e) = fs::remove_file(&homonymous_bin) {
        if e.kind() != std::io::ErrorKind::NotFound {
            critical_errors.push(format!("Failed to remove binary '{}': {}", homonymous_bin.display(), e));
        }
    }

    if let Ok(mut child) = Command::new("update-desktop-database")
        .arg(&config.apps_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn() 
    {
        if let Err(e) = child.wait() {
            tracing::warn!("Failed to wait for update-desktop-database: {}", e);
        }
    } else if !silent {
        tracing::warn!("Failed to launch update-desktop-database.");
    }

    if let Err(e) = std::process::Command::new("touch")
        .arg(&config.apps_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        tracing::warn!("Failed to refresh app dir mtime '{}': {}", config.apps_dir.display(), e);
    }

    if !critical_errors.is_empty() {
        for err in &critical_errors {
            if !silent { error_msg(err); }
            tracing::error!("{}", err);
        }
        return Err(crate::error::KoreError::Generic(format!(
            "Removal failed with {} critical error(s).",
            critical_errors.len()
        )));
    }

    if !silent { success_msg(&format!("{} successfully removed!", target_path.file_name().unwrap_or_default().to_string_lossy())); }
    if !is_cli {
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    tracing::info!(operation = "remove", app = app_name, "Remove flow finished");

    Ok(())
}
