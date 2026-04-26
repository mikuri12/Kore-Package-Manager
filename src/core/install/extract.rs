use crate::config::Config;
use crate::utils::{error_msg, find_executables, find_bundled_desktop_files};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn extract_and_scan(
    config: &Config,
    tarball: &Path,
    target_folder_name: Option<&str>,
    silent: bool,
) -> Result<Option<(PathBuf, String, Vec<PathBuf>, Vec<PathBuf>)>, crate::error::KoreError> {
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
