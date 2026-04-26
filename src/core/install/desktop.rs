use crate::config::Config;
use crate::utils::{error_msg, find_icon, success_msg};
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::core::install::utils::find_desktop_files_with_target;

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

pub fn finalize_installation(
    config: &Config,
    target: &Path,
    exec_path: &Path,
    app_name: &str,
    use_root: bool,
    use_terminal: bool,
    category: &str,
    bundled_desktop: Option<PathBuf>,
    silent: bool,
) -> Result<(), crate::error::KoreError> {
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

    let desktop_content = if let Some(bd_path) = bundled_desktop {
        let content = fs::read_to_string(bd_path).unwrap_or_default();
        let mut new_lines = Vec::new();
        let mut has_terminal = false;
        for line in content.lines() {
            if line.trim_start().starts_with("Exec=") {
                new_lines.push(format!("Exec={}", final_exec));
            } else if line.trim_start().starts_with("TryExec=") {
                new_lines.push(format!("TryExec={}", final_exec));
            } else if line.trim_start().starts_with("Icon=") {
                new_lines.push(format!("Icon={}", icon_path));
            } else if line.trim_start().starts_with("Terminal=") {
                new_lines.push(format!("Terminal={}", if use_terminal { "true" } else { "false" }));
                has_terminal = true;
            } else {
                new_lines.push(line.to_string());
            }
        }
        if !has_terminal {
            new_lines.push(format!("Terminal={}", if use_terminal { "true" } else { "false" }));
        }
        new_lines.join("\n")
    } else {
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
            if use_terminal { "true" } else { "false" },
            target.display(),
            category
        )
    };

    let desktop_path = config.apps_dir.join(format!("{}.desktop", app_name.to_lowercase().replace(' ', "-")));
    fs::write(desktop_path, desktop_content)?;

    // Refresh desktop database to show the icon and entry immediately
    let _ = Command::new("update-desktop-database").arg(&config.apps_dir).status();
    let _ = Command::new("touch").arg(&config.apps_dir).status();

    Ok(())
}
