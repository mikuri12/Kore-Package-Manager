use crate::config::Config;
use crate::utils::{error_msg, find_executables, find_bundled_desktop_files, find_icon, info_msg, success_msg, is_gui_app};
use dialoguer::{Select, Confirm, Input};
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

pub enum InstallMessage {
    Progress(String, f64),
    SelectAsset(Vec<String>, tokio::sync::oneshot::Sender<usize>),
    SelectBinary(Vec<String>, tokio::sync::oneshot::Sender<usize>),
    SelectDesktop(Vec<String>, tokio::sync::oneshot::Sender<usize>),
}
fn find_desktop_files_with_target(config: &Config, target_str: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    if let Ok(entries) = fs::read_dir(&config.apps_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if content.contains(target_str) {
                        found.push(path);
                    }
                }
            }
        }
    }
    found
}

pub fn list_cli(config: &Config) {
    if let Ok(entries) = fs::read_dir(&config.install_dir) {
        let mut apps = Vec::new();
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                apps.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        
        if apps.is_empty() {
            error_msg("No installed applications.");
        } else {
            println!("\x1b[1;36mInstalled Apps:\x1b[0m");
            for app in apps {
                println!("  - {}", app);
            }
        }
    } else {
        error_msg("Could not read installation directory.");
    }
}

pub fn update_desktop_file(config: &Config, app_folder: &str, new_val: &str, field: &str, silent: bool) {
    let target_str = config.install_dir.join(app_folder).to_string_lossy().to_string();
    let found_path = find_desktop_files_with_target(config, &target_str).into_iter().next();

    if let Some(desktop_file) = found_path {
        let mut final_val = new_val.to_string();
        if field == "Categories" && !final_val.ends_with(';') {
            final_val.push(';');
        }

        if let Ok(file_raw) = fs::File::open(&desktop_file) {
            let reader = BufReader::new(file_raw);
            let mut new_lines = Vec::new();
            for line in reader.lines().flatten() {
                if line.starts_with(&format!("{}=", field)) {
                    new_lines.push(format!("{}={}", field, final_val));
                } else {
                    new_lines.push(line);
                }
            }
            if fs::write(&desktop_file, new_lines.join("\n")).is_ok() {
                // Simulate a directory change to refresh system menus
                let _ = std::process::Command::new("touch").arg(&config.apps_dir).status();
                if !silent { success_msg(&format!("Field {} updated to: {}", field, final_val)); }
                return;
            }
        }
        if !silent { error_msg("Could not write the .desktop file"); }
    } else {
        if !silent { error_msg(&format!("Could not find shortcut linked to the folder {}", app_folder)); }
    }
}

pub fn update_exec_modifiers(
    config: &Config,
    app_folder: &str,
    new_root: Option<bool>,
    new_env: Option<String>,
    silent: bool,
) {
    let target_str = config.install_dir.join(app_folder).to_string_lossy().to_string();
    let found_path = find_desktop_files_with_target(config, &target_str).into_iter().next();

    if let Some(desktop_file) = found_path {
        let mut current_exec = String::new();
        if let Ok(file_raw) = fs::File::open(&desktop_file) {
            let reader = BufReader::new(file_raw);
            for line in reader.lines().flatten() {
                if line.starts_with("Exec=") {
                    current_exec = line["Exec=".len()..].to_string();
                    break;
                }
            }
        }

        if current_exec.is_empty() {
            if !silent { error_msg("Exec line not found in the .desktop file."); }
            return;
        }

        let mut is_root = false;
        let mut env_vars = String::new();
        let base_bin = config.bin_dir.join(app_folder).to_string_lossy().to_string();

        let parts = current_exec.split_whitespace();
        for part in parts {
            if part == "pkexec" {
                is_root = true;
            } else if part == "env" {
                continue;
            } else if part.contains('=') {
                env_vars.push_str(part);
                env_vars.push(' ');
            }
        }

        env_vars = env_vars.trim().to_string();

        if let Some(r) = new_root {
            is_root = r;
        }
        if let Some(e) = new_env {
            env_vars = e.trim().to_string();
        }

        let mut new_exec = String::new();
        if !env_vars.is_empty() {
            new_exec.push_str("env ");
            new_exec.push_str(&env_vars);
            new_exec.push(' ');
        }
        if is_root {
            new_exec.push_str("pkexec ");
        }
        new_exec.push_str(&base_bin);

        update_desktop_file(config, app_folder, &new_exec, "Exec", silent);

    } else {
        if !silent { error_msg(&format!("Could not find shortcut linked to the folder {}", app_folder)); }
    }
}

pub fn extract_and_scan(
    config: &Config,
    tarball: &Path,
    target_folder_name: Option<&str>,
    silent: bool,
) -> Result<Option<(PathBuf, String, Vec<PathBuf>, Vec<PathBuf>)>, crate::error::TmError> {
    if !tarball.exists() || !tarball.is_file() {
        if !silent { error_msg(&format!("The file '{}' does not exist.", tarball.display())); }
        return Ok(None);
    }

    let file_name = tarball.file_name().unwrap_or_default().to_string_lossy();
    let is_zip = file_name.ends_with(".zip");
    
    let raw_name_folder = target_folder_name.map(|s| s.to_string()).unwrap_or_else(|| {
        file_name
            .replace(".tar.gz", "")
            .replace(".tar.xz", "")
            .replace(".tar.bz2", "")
            .replace(".zip", "")
    });

    let target = config.install_dir.join(&raw_name_folder);

    if target.exists() {
        let _ = fs::remove_dir_all(&target); // Try to blind delete to overwrite if it already existed
    }

    fs::create_dir_all(&target)?;
    
    let success = if is_zip {
        Command::new("unzip")
            .args(["-q", tarball.to_str().unwrap(), "-d", target.to_str().unwrap()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        let output = Command::new("tar").arg("-tf").arg(tarball).output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        let mut first_components = std::collections::HashSet::new();
        for line in stdout.lines() {
            if line.is_empty() { continue; }
            let parts: Vec<&str> = line.split('/').collect();
            if !parts.is_empty() {
                first_components.insert(parts[0]);
            }
        }
        
        let mut should_strip = false;
        if first_components.len() == 1 {
            let root = first_components.into_iter().next().unwrap();
            if stdout.lines().any(|l| l.starts_with(&format!("{}/", root))) {
                should_strip = true;
            }
        }

        let mut tar_args = vec!["-xf", tarball.to_str().unwrap(), "-C", target.to_str().unwrap()];
        if should_strip {
            tar_args.push("--strip-components=1");
        }

        Command::new("tar")
            .args(&tar_args)
            .status()?
            .success()
    };

    if !success {
        let _ = fs::remove_dir_all(&target); // Clean up residue on failure
        return Ok(None);
    }

    let executables = find_executables(&target, 5);
    let desktop_files = find_bundled_desktop_files(&target, 5);
    Ok(Some((target, raw_name_folder, executables, desktop_files)))
}

pub fn finalize_installation(
    config: &Config,
    target: &Path,
    exec_path: &Path,
    app_name: &str,
    use_root: bool,
    category: &str,
    bundled_desktop: Option<PathBuf>,
    silent: bool,
) -> Result<(), crate::error::TmError> {
    let exec_name = exec_path.file_name().unwrap_or_default().to_string_lossy();
    let icon_path = find_icon(target, app_name, &exec_name).unwrap_or_else(|| "utilities-terminal".to_string());

    let sanitized_name = target.file_name().unwrap_or_default().to_string_lossy().to_string();
    let bin_dest = config.bin_dir.join(&sanitized_name);
    if bin_dest.exists() {
        let _ = fs::remove_file(&bin_dest);
    }
    
    if let Err(e) = symlink(exec_path, &bin_dest) {
         if !silent { error_msg(&format!("Unable to create symlink: {}", e)); }
         return Ok(());
    }

    let mut final_exec = bin_dest.to_string_lossy().to_string();
    if use_root {
        final_exec = format!("pkexec {}", final_exec);
    }

    let desktop_content = if let Some(bd_path) = bundled_desktop {
        let content = fs::read_to_string(bd_path).unwrap_or_default();
        let mut new_lines = Vec::new();
        for line in content.lines() {
            if line.trim_start().starts_with("Exec=") {
                new_lines.push(format!("Exec={}", final_exec));
            } else if line.trim_start().starts_with("TryExec=") {
                new_lines.push(format!("TryExec={}", final_exec));
            } else if line.trim_start().starts_with("Icon=") {
                new_lines.push(format!("Icon={}", icon_path));
            } else {
                new_lines.push(line.to_string());
            }
        }
        new_lines.join("\n")
    } else {
        let is_terminal = !is_gui_app(exec_path);
        format!(
r#"[Desktop Entry]
Name={}
Exec={}
Icon={}
Type=Application
Terminal={}
Path={}
Categories={};"#,
            app_name,
            final_exec,
            icon_path,
            if is_terminal { "true" } else { "false" },
            target.display(),
            category
        )
    };

    let desktop_path = config.apps_dir.join(format!("{}.desktop", sanitized_name));
    fs::write(desktop_path, desktop_content)?;

    // Refresh desktop database to show the icon and entry immediately
    let _ = Command::new("update-desktop-database").arg(&config.apps_dir).status();
    let _ = Command::new("touch").arg(&config.apps_dir).status();

    Ok(())
}

pub async fn install_app(
    config: &Config,
    source: &str,
    app_name_opt: Option<&str>,
    use_root_opt: Option<&str>,
    category_opt: Option<&str>,
    is_cli: bool,
    tx: Option<tokio::sync::mpsc::UnboundedSender<InstallMessage>>,
) -> Result<(), crate::error::TmError> {
    let mut actual_tarball = PathBuf::from(source);
    let mut downloaded = false;
    let mut repo_name_opt: Option<String> = None;
    let mut repo_package_name_opt: Option<String> = None;
    let mut repo_category_opt: Option<String> = None;
    let mut repo_requires_root_opt: Option<bool> = None;

    if !actual_tarball.exists() {
        // Try to match it to a repository
        let all_repos = crate::core::repo::get_all_repos(config);
        if let Some(repo_source) = all_repos.iter().find(|r| r.repo.name.to_lowercase() == source.to_lowercase() || (!r.repo.package_name.is_empty() && r.repo.package_name.to_lowercase() == source.to_lowercase())) {
            repo_name_opt = Some(repo_source.repo.name.clone());
            if !repo_source.repo.package_name.is_empty() {
                repo_package_name_opt = Some(repo_source.repo.package_name.clone());
            } else {
                repo_package_name_opt = Some(repo_source.repo.name.clone());
            }
            repo_category_opt = Some(repo_source.repo.category.clone());
            repo_requires_root_opt = Some(repo_source.repo.requires_root);

            let url = &repo_source.repo.url;
            if crate::core::download::is_supported_git_url(url) {
                if is_cli { info_msg(&format!("Fetching releases for {}...", repo_source.repo.name)); }
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
                                            return Err(crate::error::TmError::Generic(e.to_string()));
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                if is_cli { error_msg(&format!("Failed to query release API: {}", e)); }
                                if let Some(t) = &tx { let _ = t.send(InstallMessage::Progress(format!("Error: {}", e), -1.0)); }
                                return Err(crate::error::TmError::Generic(e.to_string()));
                            }
                        }
            } else {
                if is_cli { info_msg(&format!("{} is not a known Git provider. Treating as direct download link...", url)); }
                
                let resolved_url = match crate::core::download::resolve_dynamic_url(url).await {
                    Ok(u) => u,
                    Err(e) => {
                        if is_cli { error_msg(&format!("Failed to resolve dynamic URL: {}", e)); }
                        return Err(crate::error::TmError::Generic(e.to_string()));
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
                        return Err(crate::error::TmError::Generic(e.to_string()));
                    }
                }
            }
        } else {
            if is_cli { error_msg(&format!("The file '{}' does not exist, and no repository matches this name.", source)); }
            if let Some(t) = &tx { let _ = t.send(InstallMessage::Progress("File or repository not found".to_string(), -1.0)); }
            return Err(crate::error::TmError::Generic("Not found".into()));
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
        }).await.map_err(|e| crate::error::TmError::Generic(e.to_string()))??;

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
                let category_clone = category.clone();
                let bundled_desktop_clone = bundled_desktop.clone();
                let is_cli_clone = is_cli;

                tokio::task::spawn_blocking(move || {
                    finalize_installation(&config_clone, &target_clone, &exec_path_clone, &app_name_clone, use_root_clone, &category_clone, bundled_desktop_clone, !is_cli_clone)
                }).await.map_err(|e| crate::error::TmError::Generic(e.to_string()))??;
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

pub fn remove_app(config: &Config, app_name: &str, is_cli: bool, silent: bool) -> Result<(), crate::error::TmError> {
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

pub async fn update_tm(config: &Config) -> Result<(), crate::error::TmError> {
    info_msg("Looking for the latest stable version on GitHub Releases...");

    let client = reqwest::Client::builder()
        .user_agent("Tarball-Manager/1.0")
        .build()?;

    let response = client.get("https://api.github.com/repos/ezequielgk/Tarball-Manager/releases/latest").send().await;

    if response.is_err() || !response.as_ref().unwrap().status().is_success() {
        error_msg("Error connecting to GitHub API.");
        return Ok(());
    }

    let release_data: Result<crate::core::download::Release, _> = response.unwrap().json().await;
    let mut latest_url = String::new();
    let mut latest_version = String::new();

    if let Ok(release) = release_data {
        latest_version = release.tag_name;
        for asset in release.assets {
            if asset.browser_download_url.ends_with("/tm") {
                latest_url = asset.browser_download_url;
                break;
            }
        }
    }

    if latest_url.is_empty() {
        error_msg("Compiled 'tm' binary not found in the latest GitHub Release.");
        return Ok(());
    }

    let current_version = env!("CARGO_PKG_VERSION");
    
    println!("\x1b[1;36mUpdate available:\x1b[0m");
    println!("  - Current version: v{}", current_version);
    println!("  - Latest version:  {}", latest_version);

    if !Confirm::new()
        .with_prompt("Do you want to proceed with the update?")
        .default(true)
        .interact()
        .unwrap_or(false)
    {
        info_msg("Update cancelled by user.");
        return Ok(());
    }

    let bin_path = config.bin_dir.join("tm");
    let temp_dir = config.bin_dir.join(".tm_update"); // Use a dir in the same filesystem
    std::fs::create_dir_all(&temp_dir)?;

    info_msg(&format!("Downloading update..."));

    match crate::core::download::download_file(&latest_url, &temp_dir, None).await {
        Ok(downloaded_file) => {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(&downloaded_file) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                let _ = fs::set_permissions(&downloaded_file, perms);
            }

            // Perform the swap
            if fs::rename(&downloaded_file, &bin_path).is_ok() {
                success_msg("tm successfully updated!");
            } else {
                // If rename fails, try copy as fallback (though rename in same fs should work)
                if fs::copy(&downloaded_file, &bin_path).is_ok() {
                    success_msg("tm successfully updated (via copy)!");
                } else {
                    error_msg("Error replacing the current binary. Make sure you have permissions.");
                }
                let _ = fs::remove_file(&downloaded_file);
            }
            let _ = fs::remove_dir_all(&temp_dir);
        }
        Err(_) => {
            error_msg("Could not download the binary from GitHub.");
            let _ = fs::remove_dir_all(&temp_dir);
        }
    }

    Ok(())
}

pub fn get_all_categories(config: &Config) -> Vec<String> {
    let mut categories: HashSet<String> = HashSet::new();
    let default_categories = ["Utility", "Network", "Game", "Development", "Graphics", "AudioVideo", "System", "Office"];
    for cat in default_categories {
        categories.insert(cat.to_string());
    }

    if let Ok(entries) = fs::read_dir(&config.apps_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                if let Ok(file_raw) = fs::File::open(&path) {
                    let reader = BufReader::new(file_raw);
                    for line in reader.lines().flatten() {
                        if line.starts_with("Categories=") {
                            let cats = line["Categories=".len()..].split(&[';', ','][..]);
                            for cat in cats {
                                let trimmed = cat.trim();
                                if !trimmed.is_empty() {
                                    categories.insert(trimmed.to_string());
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    let mut sorted: Vec<String> = categories.into_iter().collect();
    sorted.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    sorted
}
