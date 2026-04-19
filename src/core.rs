use crate::config::Config;
use crate::utils::{error_msg, find_executables, find_icon, info_msg, success_msg};
use dialoguer::{Select, Confirm, Input};
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    let mut found_path = None;

    if let Ok(entries) = fs::read_dir(&config.apps_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if content.contains(&target_str) {
                        found_path = Some(path);
                        break;
                    }
                }
            }
        }
    }

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
    let mut found_path = None;

    if let Ok(entries) = fs::read_dir(&config.apps_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if content.contains(&target_str) {
                        found_path = Some(path);
                        break;
                    }
                }
            }
        }
    }

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
    silent: bool,
) -> anyhow::Result<Option<(PathBuf, String, Vec<PathBuf>)>> {
    if !tarball.exists() || !tarball.is_file() {
        if !silent { error_msg(&format!("The file '{}' does not exist.", tarball.display())); }
        return Ok(None);
    }

    let file_name = tarball.file_name().unwrap_or_default().to_string_lossy();
    let raw_name_folder = file_name
        .replace(".tar.gz", "")
        .replace(".tar.xz", "")
        .replace(".tar.bz2", "");

    let target = config.install_dir.join(&raw_name_folder);

    if target.exists() {
        let _ = fs::remove_dir_all(&target); // Try to blind delete to overwrite if it already existed
    }

    fs::create_dir_all(&target)?;
    
    let tar_status = Command::new("tar")
        .args(["-xf", tarball.to_str().unwrap(), "-C", target.to_str().unwrap(), "--strip-components=1"])
        .status()?;

    if !tar_status.success() {
        let _ = fs::remove_dir_all(&target); // Clean up residue on failure
        return Ok(None);
    }

    let executables = find_executables(&target, 3);
    Ok(Some((target, raw_name_folder, executables)))
}

pub fn finalize_installation(
    config: &Config,
    target: &Path,
    exec_path: &Path,
    app_name: &str,
    use_root: bool,
    category: &str,
    silent: bool,
) -> anyhow::Result<()> {
    let exec_name = exec_path.file_name().unwrap_or_default().to_string_lossy();
    let icon_path = find_icon(target, app_name, &exec_name).unwrap_or_else(|| "utilities-terminal".to_string());

    let bin_dest = config.bin_dir.join(app_name);
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

    let desktop_content = format!(
r#"[Desktop Entry]
Name={}
Exec={}
Icon={}
Type=Application
Terminal=false
Path={}
Categories={};"#,
        app_name,
        final_exec,
        icon_path,
        target.display(),
        category
    );

    let desktop_path = config.apps_dir.join(format!("{}.desktop", app_name));
    fs::write(desktop_path, desktop_content)?;

    Ok(())
}

pub fn install_app(
    config: &Config,
    source: &str,
    app_name_opt: Option<&str>,
    use_root_opt: Option<&str>,
    category_opt: Option<&str>,
    is_cli: bool,
) -> anyhow::Result<()> {
    let mut actual_tarball = PathBuf::from(source);
    let mut downloaded = false;

    if !actual_tarball.exists() {
        // Try to match it to a repository
        let all_repos = crate::repo::get_all_repos(config);
        if let Some(repo_source) = all_repos.iter().find(|r| r.repo.name.to_lowercase() == source.to_lowercase()) {
            let url = &repo_source.repo.url;
            if crate::download::is_supported_git_url(url) {
                info_msg(&format!("Fetching releases for {}...", repo_source.repo.name));
                match crate::download::get_latest_release_assets(url) {
                    Ok(assets) => {
                        if assets.is_empty() {
                            error_msg("No suitable tarball assets found in the latest release.");
                            return Ok(());
                        }
                        
                        let selected_asset = if assets.len() == 1 {
                            &assets[0]
                        } else {
                            let choices: Vec<String> = assets.iter().map(|a| a.name.clone()).collect();
                            info_msg("Multiple tarballs found. Please select one:");
                            let sel = Select::new().with_prompt("Tarball").items(&choices).default(0).interact().unwrap_or(0);
                            &assets[sel]
                        };
                        
                        let tmp_dir = std::env::temp_dir().join("tm_downloads");
                        std::fs::create_dir_all(&tmp_dir)?;
                        
                        info_msg(&format!("Downloading {}...", selected_asset.name));
                        match crate::download::download_file(&selected_asset.browser_download_url, &tmp_dir) {
                            Ok(path) => {
                                actual_tarball = path;
                                downloaded = true;
                                info_msg("Download complete!");
                            }
                            Err(e) => {
                                error_msg(&format!("Failed to download: {}", e));
                                return Ok(());
                            }
                        }
                    }
                    Err(e) => {
                        error_msg(&format!("Failed to query release API: {}", e));
                        return Ok(());
                    }
                }
            } else {
                info_msg(&format!("{} is not a known Git provider. Treating as direct download link...", url));
                let tmp_dir = std::env::temp_dir().join("tm_downloads");
                std::fs::create_dir_all(&tmp_dir)?;
                
                info_msg(&format!("Downloading from {}...", url));
                match crate::download::download_file(url, &tmp_dir) {
                    Ok(path) => {
                        actual_tarball = path;
                        downloaded = true;
                        info_msg("Download complete!");
                    }
                    Err(e) => {
                        error_msg(&format!("Failed to download: {}", e));
                        return Ok(());
                    }
                }
            }
        } else {
            error_msg(&format!("The file '{}' does not exist, and no repository matches this name.", source));
            return Ok(());
        }
    }

    if let Some((target, raw_name_folder, executables)) = extract_and_scan(config, &actual_tarball, false)? {
        let exec_path = if executables.is_empty() {
            error_msg("No executable binary found.");
            return Ok(());
        } else if executables.len() == 1 {
            executables[0].clone()
        } else {
            info_msg("Select the main executable binary:");
            let choices: Vec<String> = executables.iter().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).collect();
            let sel = Select::new().with_prompt("Binary").items(&choices).default(0).interact().unwrap_or(0);
            executables[sel].clone()
        };

        let app_name = app_name_opt
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                // If it was downloaded from a repo, use the repo name by default!
                if downloaded {
                    source.to_string()
                } else {
                    raw_name_folder.clone()
                }
            });

        // Use root from the repo if matched!
        let use_root = if downloaded && use_root_opt.is_none() {
            let all_repos = crate::repo::get_all_repos(config);
            let requires_root = all_repos.iter()
                .find(|r| r.repo.name.to_lowercase() == source.to_lowercase())
                .map(|r| r.repo.requires_root)
                .unwrap_or(false);
            requires_root
        } else {
            use_root_opt
                .map(|s| s.to_lowercase() == "si" || s.to_lowercase() == "yes" || s.to_lowercase() == "s")
                .unwrap_or(false)
        };

        let category = if downloaded && category_opt.is_none() {
            let all_repos = crate::repo::get_all_repos(config);
            all_repos.iter()
                .find(|r| r.repo.name.to_lowercase() == source.to_lowercase())
                .map(|r| r.repo.category.clone())
                .unwrap_or_else(|| "Utility".to_string())
        } else {
            category_opt.unwrap_or("Utility").to_string()
        };
        
        finalize_installation(config, &target, &exec_path, &app_name, use_root, &category, false)?;
        if !is_cli {
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    }
    
    // Clean up downloaded file
    if downloaded && actual_tarball.exists() {
        let _ = std::fs::remove_file(&actual_tarball);
    }
    
    Ok(())
}

pub fn remove_app(config: &Config, app_name: &str, is_cli: bool, silent: bool) -> anyhow::Result<()> {
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
    if let Ok(entries) = fs::read_dir(&config.apps_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if content.contains(&target_str) {
                        let app_name_from_file = path.file_stem().unwrap_or_default();
                        let associated_bin = config.bin_dir.join(app_name_from_file);
                        let _ = fs::remove_file(&associated_bin);
                        let _ = fs::remove_file(&path);
                        if !silent { success_msg(&format!("Removed shortcut: {}", path.file_name().unwrap_or_default().to_string_lossy())); }
                    }
                }
            }
        }
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

pub fn update_tm(config: &Config) -> anyhow::Result<()> {
    info_msg("Looking for the latest stable version on GitHub Releases...");

    let curl_output = Command::new("curl")
        .args(["-s", "https://api.github.com/repos/ezequielgk/Tarball-Manager/releases/latest"])
        .output()?;

    if !curl_output.status.success() {
        error_msg("Error connecting to GitHub API.");
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&curl_output.stdout);
    let mut latest_url = String::new();
    for line in stdout.lines() {
        if line.contains("browser_download_url") && line.contains("/tm\"") {
            if let Some(start) = line.find("https://") {
                if let Some(end) = line[start..].find('"') {
                    latest_url = line[start..start + end].to_string();
                    break;
                }
            }
        }
    }

    if latest_url.is_empty() {
        error_msg("Compiled 'tm' binary not found in the latest GitHub Release.");
        return Ok(());
    }

    let bin_path = config.bin_dir.join("tm");
    let temp_bin = config.bin_dir.join("tm.tmp");

    info_msg(&format!("Downloading from: {}", latest_url));

    let download_status = Command::new("curl")
        .args(["-sSL", &latest_url, "-o"])
        .arg(&temp_bin)
        .status()?;

    if !download_status.success() {
        error_msg("Could not download the binary from GitHub.");
        let _ = fs::remove_file(&temp_bin);
        return Ok(());
    }

    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = fs::metadata(&temp_bin) {
        let mut perms = metadata.permissions();
        perms.set_mode(0o755);
        let _ = fs::set_permissions(&temp_bin, perms);
    }

    if fs::rename(&temp_bin, &bin_path).is_ok() {
        success_msg("tm successfully updated!");
    } else {
        error_msg("Error replacing the current binary.");
        let _ = fs::remove_file(&temp_bin);
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
