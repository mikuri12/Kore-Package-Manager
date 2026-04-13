use crate::config::Config;
use crate::utils::{error_msg, find_executables, find_icon, info_msg, success_msg};
use dialoguer::{Select, Confirm, Input};
use std::fs;
use std::io::{self, BufRead, BufReader};
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
                println!("  󰏗 {}", app);
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
                // Simula un cambio en el directorio para refrescar los menús del sistema
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

pub fn extract_and_scan(
    config: &Config,
    tarball: &Path,
    silent: bool,
) -> io::Result<Option<(PathBuf, String, Vec<PathBuf>)>> {
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
        let _ = fs::remove_dir_all(&target); // Intenta borrar ciegamente para sobreescribir si ya existiera
    }

    fs::create_dir_all(&target)?;
    
    let tar_status = Command::new("tar")
        .args(["-xf", tarball.to_str().unwrap(), "-C", target.to_str().unwrap(), "--strip-components=1"])
        .status()?;

    if !tar_status.success() {
        let _ = fs::remove_dir_all(&target); // Limpieza de residuos por fallo
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
) -> io::Result<()> {
    let icon_path = find_icon(target, 4).unwrap_or_else(|| "utilities-terminal".to_string());

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
    tarball: &Path,
    app_name_opt: Option<&str>,
    use_root_opt: Option<&str>,
    category_opt: Option<&str>,
    is_cli: bool,
) -> io::Result<()> {
    if let Some((target, raw_name_folder, executables)) = extract_and_scan(config, tarball, false)? {
        let exec_path = if executables.is_empty() {
            error_msg("No executable binary found.");
            return Ok(());
        } else if executables.len() == 1 {
            executables[0].clone()
        } else {
            info_msg("Select the main executable binary:");
            let choices: Vec<String> = executables.iter().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).collect();
            let sel = Select::new().with_prompt("󰜎 Binary").items(&choices).default(0).interact().unwrap_or(0);
            executables[sel].clone()
        };

        let app_name = app_name_opt
            .map(|s| s.to_string())
            .unwrap_or_else(|| raw_name_folder.clone());

        let use_root = use_root_opt
            .map(|s| s.to_lowercase() == "si" || s.to_lowercase() == "yes" || s.to_lowercase() == "s")
            .unwrap_or(false);

        let category = category_opt.unwrap_or("Utility");
        
        finalize_installation(config, &target, &exec_path, &app_name, use_root, category, false)?;
        if !is_cli {
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    }
    
    Ok(())
}

pub fn remove_app(config: &Config, app_name: &str, is_cli: bool, silent: bool) -> io::Result<()> {
    let mut target_path = config.install_dir.join(app_name);

    if !target_path.exists() {
        // Sugerir nombre si no existe exactamente (case insensitive o parecido)
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
    // Localiza e iteractiva por los .desktop que contengan esta ruta
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
    // Elimina adicionalmente un binario homónimo directo en caso de enlaces rotos
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
