use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use walkdir::WalkDir;

use crate::config::Config;
use crate::utils::find_executables;

pub fn calculate_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

pub fn format_size(size: u64) -> String {
    let mb = size as f64 / 1_048_576.0;
    if mb > 1024.0 {
        format!("{:.2} GB", mb / 1024.0)
    } else {
        format!("{:.2} MB", mb)
    }
}

pub fn generate_preview(config: &Config, app_name: &str) -> String {
    let target = config.install_dir.join(app_name);
    let size = calculate_size(&target);
    let size_str = format_size(size);

    let associated_bin = config.bin_dir.join(app_name);
    let bin_str = if associated_bin.exists() {
        if let Ok(target_link) = fs::read_link(&associated_bin) {
            format!("Symlink to: {}", target_link.display())
        } else {
            "Referenced binary (unknown)".to_string()
        }
    } else {
        let execs = find_executables(&target, 3);
        if execs.is_empty() {
            "No bin tracker".to_string()
        } else {
            format!("Potential binary: {}", execs[0].file_name().unwrap_or_default().to_string_lossy())
        }
    };

    let mut preview = format!("--- DETAILS ---\n");
    preview.push_str(&format!("Size: {}\n", size_str));
    preview.push_str(&format!("Binary: {}\n", bin_str));
    preview.push_str("\n--- CONTENT ---\n");

    if let Ok(entries) = fs::read_dir(&target) {
        let mut count = 0;
        let mut files = Vec::new();
        for entry in entries.flatten().take(15) {
            let name = entry.file_name().to_string_lossy().to_string();
            let suffix = if entry.path().is_dir() { "/" } else { "" };
            files.push(format!("{}{}", name, suffix));
            count += 1;
        }

        files.sort();
        for f in files {
            preview.push_str(&format!("{}\n", f));
        }

        if count == 15 {
            preview.push_str("... (more files)\n");
        }
    }

    preview
}

pub fn generate_archive_preview(file_path: &Path) -> String {
    let mut preview = format!("--- ARCHIVE DETAILS ---\n");
    if let Ok(metadata) = fs::metadata(file_path) {
        preview.push_str(&format!("Size: {}\n", format_size(metadata.len())));
    }
    preview.push_str("\n--- LIMITED PREVIEW ---\n");
    
    let is_zip = file_path.to_string_lossy().ends_with(".zip");
    
    let child_res = if is_zip {
        Command::new("unzip")
            .args(["-Z1", file_path.to_str().unwrap()])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
    } else {
        Command::new("tar")
            .args(["-tf", file_path.to_str().unwrap()])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
    };
    
    if let Ok(mut child) = child_res {
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().take(15) {
                if let Ok(l) = line {
                    preview.push_str(&format!("{}\n", l));
                }
            }
        }
        let _ = child.kill(); 
        let _ = child.wait();
    }
    
    preview
}
