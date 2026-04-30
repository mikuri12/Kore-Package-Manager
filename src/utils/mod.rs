use colored::*;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub mod fs;

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

pub fn find_bundled_desktop_files(target: &Path, max_depth: usize) -> Vec<PathBuf> {
    WalkDir::new(target)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("desktop"))
        .map(|e| e.path().to_path_buf())
        .collect()
}

pub fn find_icon(target: &Path, app_name: &str, exec_name: &str) -> Option<String> {
    let mut best_icon: Option<String> = None;
    let mut max_score = -1;
    let app_lower = app_name.to_lowercase();
    let app_sanitized = app_lower.replace(" ", "-");
    let exec_lower = exec_name.to_lowercase()
        .replace(".appimage", "")
        .replace(".exe", "")
        .replace(".bin", "");
    let target_name = target.file_name().unwrap_or_default().to_string_lossy().to_lowercase();

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
                    
                    if file_stem == app_lower || file_stem == app_sanitized || file_stem == target_name {
                        score += 100;
                    } else if (file_stem.contains(&app_lower) || file_stem.contains(&app_sanitized) || file_stem.contains(&target_name)) && target_name.len() > 2 {
                        score += 40;
                    }

                    if file_stem == exec_lower {
                        score += 50;
                    } else if file_stem.contains(&exec_lower) && exec_lower.len() > 3 {
                        score += 20;
                    }

                    if path_str.contains(&app_lower) || path_str.contains(&app_sanitized) || path_str.contains(&target_name) {
                        score += 30;
                    }

                    if path_str.contains("/icons/") || path_str.contains("/hicolor/") || path_str.contains("/scalable/") || path_str.contains("/pixmaps/") || path_str.contains("/share/icons/") {
                        score += 80;
                    }

                    if file_stem == "icon" || file_stem == "logo" || file_stem == "app" || file_stem == "main" || file_stem == "code" || file_stem == "default" || file_stem == "favicon" || file_stem == "launcher" || file_stem == "brand" {
                        score += 80;
                    } else if file_stem.contains("icon") || file_stem.contains("logo") || file_stem.contains("default") || file_stem.contains("main") || file_stem.contains("app") {
                        score += 20;
                    }

                    let size_score = file_stem.chars()
                        .filter(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse::<i32>()
                        .unwrap_or(0);
                    
                    if size_score > 0 {
                        score += size_score / 32;
                    }

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


pub fn is_gui_app(executable: &Path) -> bool {
    if let Ok(output) = std::process::Command::new("ldd").arg(executable).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let gui_libs = ["libgtk", "libQt", "libX11", "libwayland", "libSDL", "libxcb", "libcairo", "libpango", "libEGL"];
        for lib in gui_libs {
            if stdout.contains(lib) {
                return true;
            }
        }
    }
    false
}
