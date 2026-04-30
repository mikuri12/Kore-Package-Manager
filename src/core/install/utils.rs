use crate::config::Config;
use crate::utils::error_msg;
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::os::unix::fs::PermissionsExt;
use anyhow::{Context, Result};

fn normalize_desktop_value(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}

fn desktop_line_references_target(line: &str, target_str: &str) -> bool {
    let trimmed = line.trim_start();

    if let Some(exec) = trimmed.strip_prefix("Exec=").or_else(|| trimmed.strip_prefix("TryExec=")) {
        let tokens = crate::core::install::tokenize_desktop_exec(exec);
        return tokens.iter().any(|t| normalize_desktop_value(t) == target_str);
    }

    if let Some(path) = trimmed.strip_prefix("Path=") {
        return normalize_desktop_value(path) == target_str;
    }

    false
}

pub fn find_desktop_files_with_target(config: &Config, target_str: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    if let Ok(entries) = fs::read_dir(&config.apps_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if content
                        .lines()
                        .any(|line| desktop_line_references_target(line, target_str))
                    {
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
                let app_name = entry.file_name().to_string_lossy().to_string();
                let manifest_path = entry.path().join(".kpm_manifest.json");
                let mut version_str = String::new();
                if let Ok(content) = fs::read_to_string(&manifest_path) {
                    if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(v) = manifest.get("version").and_then(|val| val.as_str()) {
                            version_str = format!(" (v{})", v);
                        }
                    }
                }
                apps.push(format!("{}{}", app_name, version_str));
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
                    for line in reader.lines().map_while(Result::ok) {
                        if let Some(categories_line) = line.strip_prefix("Categories=") {
                            let cats = categories_line.split(&[';', ','][..]);
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
    sorted.sort_by_key(|a| a.to_lowercase());
    sorted
}

/// Prepara un binario para su ejecución, asegurando permisos 0o755.
/// Parchea el shebang si es necesario (ej. python a python3) para mantener la pureza del binario sin wrappers.
/// Devuelve una tupla: (Ruta_del_ejecutable, Nombre_agnóstico_del_comando)
pub fn process_binary_extension<P: AsRef<Path>>(file_path: P) -> Result<(PathBuf, String)> {
    let path = file_path.as_ref();

    let stem = path
        .file_stem()
        .context("El path no tiene un nombre de archivo válido")?
        .to_string_lossy()
        .to_string();

    // Fix legacy shebangs (e.g. #!/usr/bin/python -> #!/usr/bin/python3)
    if let Ok(mut file) = fs::File::open(path) {
        let mut buffer = [0; 128];
        use std::io::Read;
        if let Ok(n) = file.read(&mut buffer) {
            let header = String::from_utf8_lossy(&buffer[..n]);
            if header.starts_with("#!") {
                let first_line = header.lines().next().unwrap_or("");
                if first_line.contains("python") && !first_line.contains("python3") {
                    let new_shebang = first_line.replace("python", "python3");
                    if let Ok(content) = fs::read(path) {
                        if let Some(newline_pos) = content.iter().position(|&b| b == b'\n') {
                            let mut new_content = new_shebang.into_bytes();
                            new_content.push(b'\n');
                            new_content.extend_from_slice(&content[newline_pos + 1..]);
                            let _ = fs::write(path, new_content);
                        }
                    }
                }
            }
        }
    }

    // Obtenemos los permisos actuales y aplicamos 0o755 (rwxr-xr-x) al archivo original
    let mut perms = fs::metadata(path)
        .context("No se pudo obtener la metadata del archivo")?
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)
        .context("No se pudieron establecer los permisos de ejecución")?;

    Ok((path.to_path_buf(), stem))
}
