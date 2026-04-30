use crate::config::Config;
use crate::utils::{error_msg, find_icon, success_msg};
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use crate::core::install::utils::find_desktop_files_with_target;

fn quote_desktop_exec_token(token: &str) -> String {
    if token.is_empty() {
        return "\"\"".to_string();
    }
    if token.chars().any(char::is_whitespace) {
        let escaped = token
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('`', "\\`")
            .replace('$', "\\$");
        format!("\"{}\"", escaped)
    } else {
        token.to_string()
    }
}

pub fn tokenize_desktop_exec(exec: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escaped = false;

    for ch in exec.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '"' => in_quotes = !in_quotes,
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}



fn create_launcher_or_symlink(exec_path: &Path, bin_dest: &Path) -> Result<(), crate::error::KoreError> {
    symlink(exec_path, bin_dest).map_err(|e| {
        crate::error::KoreError::Generic(format!(
            "Unable to create symlink '{}' -> '{}': {}",
            bin_dest.display(),
            exec_path.display(),
            e
        ))
    })
}

pub fn update_desktop_file(config: &Config, app_folder: &str, new_val: &str, field: &str, silent: bool) {
    tracing::info!(operation = "desktop_update", app = app_folder, field = field, "Updating desktop field");
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
            for line in reader.lines().map_while(Result::ok) {
                if line.starts_with(&format!("{}=", field)) {
                    new_lines.push(format!("{}={}", field, final_val));
                } else {
                    new_lines.push(line);
                }
            }
            if fs::write(&desktop_file, new_lines.join("\n")).is_ok() {
                let _ = std::process::Command::new("touch")
                    .arg(&config.apps_dir)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                if !silent { success_msg(&format!("Field {} updated to: {}", field, final_val)); }
                tracing::info!(operation = "desktop_update", app = app_folder, field = field, file = %desktop_file.display(), "Desktop field updated");
                return;
            }
        }
        if !silent { error_msg("Could not write the .desktop file"); }
        tracing::error!(operation = "desktop_update", app = app_folder, field = field, "Could not write desktop file");
    } else {
        if !silent { error_msg(&format!("Could not find shortcut linked to the folder {}", app_folder)); }
        tracing::warn!(operation = "desktop_update", app = app_folder, field = field, "Desktop file not found for app");
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
            for line in reader.lines().map_while(Result::ok) {
                if let Some(exec) = line.strip_prefix("Exec=") {
                    current_exec = exec.to_string();
                    break;
                }
            }
        }

        if current_exec.is_empty() {
            if !silent { error_msg("Exec line not found in the .desktop file."); }
            return;
        }

        let mut is_root = false;
        let mut env_vars: Vec<String> = Vec::new();
        let base_bin = config.bin_dir.join(app_folder).to_string_lossy().to_string();

        for part in tokenize_desktop_exec(&current_exec) {
            if part == "pkexec" {
                is_root = true;
            } else if part == "env" {
                continue;
            } else if part.contains('=') {
                env_vars.push(part);
            }
        }

        if let Some(r) = new_root {
            is_root = r;
        }
        if let Some(e) = new_env {
            env_vars = if e.trim().is_empty() {
                Vec::new()
            } else {
                tokenize_desktop_exec(e.trim())
            };
        }

        let mut new_exec = String::new();
        if !env_vars.is_empty() {
            new_exec.push_str("env ");
            let rendered = env_vars
                .iter()
                .map(|v| quote_desktop_exec_token(v))
                .collect::<Vec<_>>()
                .join(" ");
            new_exec.push_str(&rendered);
            new_exec.push(' ');
        }
        if is_root {
            new_exec.push_str("pkexec ");
        }
        new_exec.push_str(&quote_desktop_exec_token(&base_bin));

        update_desktop_file(config, app_folder, &new_exec, "Exec", silent);

    } else {
        if !silent { error_msg(&format!("Could not find shortcut linked to the folder {}", app_folder)); }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn finalize_installation(
    config: &Config,
    target: &Path,
    exec_path: &Path,
    app_name: &str,
    display_name: &str,
    use_root: bool,
    use_terminal: bool,
    category: &str,
    bundled_desktop: Option<PathBuf>,
    version: Option<String>,
    asset_name: Option<String>,
    silent: bool,
) -> Result<(), crate::error::KoreError> {
    tracing::info!(operation = "desktop_finalize", app = app_name, "Finalizing desktop integration");

    let (processed_exec_path, bin_name) = crate::core::install::utils::process_binary_extension(exec_path).unwrap_or_else(|e| {
        tracing::warn!("Failed to process binary extension for {:?}: {}", exec_path, e);
        if !silent {
            error_msg(&format!("Failed to process binary extension: {}", e));
        }
        let fallback_stem = exec_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        (exec_path.to_path_buf(), fallback_stem)
    });
    
    let exec_name = processed_exec_path.file_name().unwrap_or_default().to_string_lossy();
    let icon_path = find_icon(target, display_name, &exec_name).unwrap_or_else(|| "utilities-terminal".to_string());

    let bin_dest = config.bin_dir.join(&bin_name);
    if bin_dest.exists() {
        fs::remove_file(&bin_dest).map_err(|e| {
            if !silent {
                error_msg(&format!("Unable to remove existing symlink: {}", e));
            }
            crate::error::KoreError::Generic(format!(
                "Failed to remove existing binary link '{}': {}",
                bin_dest.display(),
                e
            ))
        })?;
    }
    
    if let Err(e) = create_launcher_or_symlink(&processed_exec_path, &bin_dest) {
         if !silent { error_msg(&format!("Unable to create launcher/link: {}", e)); }
         return Err(e);
    }

    let mut final_exec = quote_desktop_exec_token(&bin_dest.to_string_lossy());
    if use_root {
        final_exec = format!("pkexec {}", final_exec);
    }

    let desktop_content = if let Some(ref bd_path) = bundled_desktop {
        let content = fs::read_to_string(bd_path).unwrap_or_default();
        let mut new_lines = Vec::new();
        let mut has_terminal = false;
        let mut has_name = false;

        for line in content.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("Exec=") {
                new_lines.push(format!("Exec={}", final_exec));
            } else if trimmed.starts_with("TryExec=") {
                new_lines.push(format!("TryExec={}", final_exec));
            } else if trimmed.starts_with("Icon=") {
                new_lines.push(format!("Icon={}", icon_path));
            } else if trimmed.starts_with("Name=") && !has_name {
                new_lines.push(format!("Name={}", display_name));
                has_name = true;
            } else if trimmed.starts_with("Terminal=") {
                new_lines.push(format!("Terminal={}", if use_terminal { "true" } else { "false" }));
                has_terminal = true;
            } else {
                new_lines.push(line.to_string());
            }
        }
        if !has_terminal {
            new_lines.push(format!("Terminal={}", if use_terminal { "true" } else { "false" }));
        }
        if !has_name {
            new_lines.insert(1, format!("Name={}", display_name));
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
            display_name,
            final_exec,
            icon_path,
            if use_terminal { "true" } else { "false" },
            quote_desktop_exec_token(&target.to_string_lossy()),
            category
        )
    };

    let desktop_path = config.apps_dir.join(format!("{}.desktop", app_name.to_lowercase().replace(' ', "-")));
    fs::write(desktop_path, desktop_content)?;

    let _ = Command::new("update-desktop-database")
        .arg(&config.apps_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = Command::new("touch")
        .arg(&config.apps_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if let Some(v) = version {
        let manifest_path = target.join(".kpm_manifest.json");
        let rel_exec = processed_exec_path.strip_prefix(target).unwrap_or(&processed_exec_path).to_string_lossy();
        let rel_desktop = bundled_desktop.as_ref().map(|d| d.strip_prefix(target).unwrap_or(d).to_string_lossy().into_owned());
        
        let mut manifest = serde_json::json!({
            "app_name": app_name,
            "version": v,
            "binary_path": rel_exec,
        });
        
        if let Some(a) = asset_name {
            manifest["asset_name"] = serde_json::Value::String(a);
        }
        if let Some(d) = rel_desktop {
            manifest["desktop_file"] = serde_json::Value::String(d);
        }
        
        let _ = fs::write(manifest_path, serde_json::to_string_pretty(&manifest).unwrap_or_default());
    }

    tracing::info!(operation = "desktop_finalize", app = app_name, "Desktop integration finalized");
    Ok(())
}
