use colored::*;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use std::sync::atomic::{AtomicBool, Ordering};

pub static IS_CLI: AtomicBool = AtomicBool::new(true);

pub fn info_msg(msg: &str) {
    tracing::info!("{}", msg);
    if IS_CLI.load(Ordering::Relaxed) {
        println!("{} {}", "[i]".cyan(), msg);
    }
}

pub fn success_msg(msg: &str) {
    tracing::info!("{}", msg);
    if IS_CLI.load(Ordering::Relaxed) {
        println!("{} {}", "[+]".green(), msg);
    }
}

pub fn error_msg(msg: &str) {
    tracing::error!("{}", msg);
    if IS_CLI.load(Ordering::Relaxed) {
        println!("{} {}", "[x]".red().bold(), msg);
    }
}


/// Busca archivos ejecutables hasta una profundidad máxima
pub fn find_executables(target: &Path, max_depth: usize) -> Vec<PathBuf> {
    WalkDir::new(target)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            if let Ok(metadata) = e.metadata() {
                metadata.permissions().mode() & 0o111 != 0
            } else {
                false
            }
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// Busca el mejor ícono coincidente explorando profundamente el directorio
pub fn find_icon(target: &Path, app_name: &str, exec_name: &str) -> Option<String> {
    let mut best_icon: Option<String> = None;
    let mut max_score = -1;
    let app_lower = app_name.to_lowercase();
    let app_sanitized = app_lower.replace(" ", "-");
    let exec_lower = exec_name.to_lowercase();

    for entry in WalkDir::new(target)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                if ext == "png" || ext == "svg" || ext == "ico" || ext == "xpm" {
                    let mut score = 0;
                    let path_str = entry.path().to_string_lossy().to_lowercase();
                    let file_stem = entry.path().file_stem().unwrap_or_default().to_string_lossy().to_lowercase();

                    if path_str.contains("node_modules") || path_str.contains("/extensions/") || path_str.contains("/.git") {
                        score -= 1000;
                    }
                    if path_str.contains("/test/") {
                        score -= 50;
                    }

                    if ext == "svg" { score += 10; }
                    if ext == "png" { score += 5; }
                    
                    if file_stem == app_lower || file_stem == app_sanitized {
                        score += 100;
                    } else if (file_stem.contains(&app_lower) || file_stem.contains(&app_sanitized)) && app_sanitized.len() > 2 {
                        score += 40;
                    }

                    if file_stem == exec_lower {
                        score += 50;
                    } else if file_stem.contains(&exec_lower) && exec_lower.len() > 3 {
                        score += 20;
                    }

                    if path_str.contains(&app_lower) || path_str.contains(&app_sanitized) {
                        score += 30;
                    }

                    if path_str.contains("/icons/") || path_str.contains("/hicolor/") || path_str.contains("/scalable/") || path_str.contains("/pixmaps/") || path_str.contains("/share/icons/") {
                        score += 80;
                    }

                    if file_stem == "icon" || file_stem == "logo" || file_stem == "app" || file_stem == "main" || file_stem == "code" || file_stem == "default" {
                        score += 40;
                    } else if file_stem.contains("icon") || file_stem.contains("logo") || file_stem.contains("default") {
                        score += 10;
                    }

                    // Boost based on size if numbers are present (e.g. default128.png)
                    let size_score = file_stem.chars()
                        .filter(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse::<i32>()
                        .unwrap_or(0);
                    
                    if size_score > 0 {
                        score += size_score / 32;
                    }

                    // Penalize deep paths lightly to prefer root icons
                    let depth = entry.path().components().count();
                    score -= (depth as i32) * 2;

                    if score > max_score {
                        max_score = score;
                        best_icon = Some(entry.path().to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    best_icon
}
