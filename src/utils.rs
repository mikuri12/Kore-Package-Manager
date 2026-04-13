use colored::*;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn info_msg(msg: &str) {
    println!("{} {}", "󰋼".cyan(), msg);
}

pub fn success_msg(msg: &str) {
    println!("{} {}", "󰄬".green(), msg);
}

pub fn error_msg(msg: &str) {
    println!("{} {}", "󰅚".red().bold(), msg);
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

/// Busca el primer ícono coincidente hasta una profundidad máxima
pub fn find_icon(target: &Path, max_depth: usize) -> Option<String> {
    for entry in WalkDir::new(target)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                if ext == "png" || ext == "svg" || ext == "ico" {
                    return Some(entry.path().to_string_lossy().to_string());
                }
            }
        }
    }
    None
}
