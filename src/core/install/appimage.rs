use crate::config::Config;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

pub fn process_appimage(
    config: &Config,
    source_file: &Path,
    target_folder_name: Option<&str>,
) -> Result<Option<(PathBuf, String, Vec<PathBuf>, Vec<PathBuf>)>, crate::error::KoreError> {
    if !source_file.exists() || !source_file.is_file() {
        return Ok(None);
    }

    let file_name = source_file.file_name().unwrap_or_default().to_string_lossy();
    let raw_name_folder = target_folder_name.map(|s| s.to_string()).unwrap_or_else(|| {
        file_name
            .replace(".AppImage", "")
            .replace(".appimage", "")
    });

    let target = config.install_dir.join(&raw_name_folder);

    if target.exists() {
        let _ = fs::remove_dir_all(&target);
    }

    fs::create_dir_all(&target)?;

    let appimage_dest = target.join(format!("{}.AppImage", raw_name_folder));
    fs::copy(source_file, &appimage_dest)?;

    let mut perms = fs::metadata(&appimage_dest)?.permissions();
    perms.set_mode(perms.mode() | 0o111);
    fs::set_permissions(&appimage_dest, perms)?;

    let temp_dir = env::temp_dir().join(format!("kpm_appimage_extract_{}", raw_name_folder));
    if temp_dir.exists() {
        let _ = fs::remove_dir_all(&temp_dir);
    }
    fs::create_dir_all(&temp_dir)?;

    // Extraemos todo el contenido para hacer una búsqueda profunda de iconos
    let _ = Command::new(&appimage_dest)
        .current_dir(&temp_dir)
        .arg("--appimage-extract")
        .output();

    let squashfs_root = temp_dir.join("squashfs-root");
    let mut best_icon = None;

    if squashfs_root.exists() {
        let dir_icon = squashfs_root.join(".DirIcon");
        if dir_icon.exists() {
            best_icon = Some(dir_icon);
        } else {
            let mut root_pngs = Vec::new();
            let mut root_icos = Vec::new();
            let lower_app_name = raw_name_folder.to_lowercase();

            for entry in WalkDir::new(&squashfs_root).into_iter().flatten() {
                let ft = entry.file_type();
                // Check if it's a file or symlink
                if ft.is_file() || ft.is_symlink() {
                    let name = entry.file_name().to_string_lossy().to_lowercase();
                    let path = entry.path().to_path_buf();
                    
                    // Ignore broken symlinks
                    if !path.exists() {
                        continue;
                    }

                    if name.ends_with(".svg") {
                        best_icon = Some(path.clone());
                        if name == "icon.svg" || name.contains("icon") || name.contains(&lower_app_name) {
                            break;
                        }
                    } else if name.ends_with(".png") {
                        root_pngs.push((name, path));
                    } else if name.ends_with(".ico") {
                        root_icos.push((name, path));
                    }
                }
            }

            if best_icon.is_none() {
                if let Some((_, path)) = root_pngs.iter().find(|(n, _)| n == "icon.png" || n.contains("icon") || n.contains(&lower_app_name)) {
                    best_icon = Some(path.clone());
                } else if !root_pngs.is_empty() {
                    best_icon = Some(root_pngs[0].1.clone());
                } else if let Some((_, path)) = root_icos.iter().find(|(n, _)| n.contains("icon") || n.contains(&lower_app_name)) {
                    best_icon = Some(path.clone());
                } else if !root_icos.is_empty() {
                    best_icon = Some(root_icos[0].1.clone());
                }
            }
        }
    }

    if let Some(icon_path) = best_icon {
        let ext = icon_path.extension().and_then(|s| s.to_str()).unwrap_or("png");
        let dest_icon = target.join(format!("{}.{}", raw_name_folder, ext));
        let _ = fs::copy(&icon_path, &dest_icon);
    }

    let _ = fs::remove_dir_all(&temp_dir);

    Ok(Some((target, raw_name_folder, vec![appimage_dest], vec![])))
}
