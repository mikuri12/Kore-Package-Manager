use crate::config::Config;
use crate::utils::{error_msg, info_msg, success_msg};
use dialoguer::Confirm;
use std::fs;

pub async fn update_kpm(config: &Config) -> Result<(), crate::error::KoreError> {
    tracing::info!(operation = "self_update", step = "start", "Starting kpm self-update");
    info_msg("Looking for the latest stable version on GitHub Releases...");

    let client = reqwest::Client::builder()
        .user_agent("Kore-Package-Manager/1.0")
        .build()?;

    let response = client
        .get("https://api.github.com/repos/ezequielgk/Kore-Package-Manager/releases/latest")
        .send()
        .await
        .map_err(|e| crate::error::KoreError::Generic(format!("Error connecting to GitHub API: {}", e)))?;

    if !response.status().is_success() {
        error_msg("Error connecting to GitHub API.");
        return Err(crate::error::KoreError::Generic(format!(
            "GitHub API returned status {}",
            response.status()
        )));
    }

    let release_data: Result<crate::core::download::Release, _> = response.json().await;
    let mut latest_url = String::new();
    let mut latest_version = String::new();

    if let Ok(release) = release_data {
        latest_version = release.tag_name;
        for asset in release.assets {
            if asset.browser_download_url.ends_with("kpm-linux-x86_64.tar.gz") {
                latest_url = asset.browser_download_url;
                break;
            }
        }
    }

    if latest_url.is_empty() {
        error_msg("Compiled 'kpm' package not found in the latest GitHub Release.");
        return Err(crate::error::KoreError::Generic(
            "Compiled package not found in latest release assets".to_string(),
        ));
    }

    let current_version = env!("CARGO_PKG_VERSION");
    
    println!("\x1b[1;36mUpdate available:\x1b[0m");
    println!("  - Current version: v{}", current_version);
    println!("  - Latest version:  {}", latest_version);

    if !Confirm::new()
        .with_prompt("Do you want to proceed with the update?")
        .default(true)
        .interact()
        .unwrap_or(false)
    {
        info_msg("Update cancelled by user.");
        tracing::info!(operation = "self_update", step = "confirm", "Update cancelled by user");
        return Ok(());
    }

    let bin_path = config.bin_dir.join("kpm");
    let temp_dir = config.install_dir.join(".kpm_update"); 
    std::fs::create_dir_all(&temp_dir)?;

    info_msg("Downloading update...");

    match crate::core::download::download_file(&latest_url, &temp_dir, None).await {
        Ok(downloaded_file) => {
            let output = std::process::Command::new("tar")
                .arg("-xzf")
                .arg(&downloaded_file)
                .arg("-C")
                .arg(&temp_dir)
                .output();

            if output.is_ok() && output.as_ref().unwrap().status.success() {
                let extracted_bin = temp_dir.join("kpm");
                if extracted_bin.exists() {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = fs::metadata(&extracted_bin) {
                        let mut perms = metadata.permissions();
                        perms.set_mode(0o755);
                        let _ = fs::set_permissions(&extracted_bin, perms);
                    }

                    if fs::rename(&extracted_bin, &bin_path).is_ok() || fs::copy(&extracted_bin, &bin_path).is_ok() {
                        success_msg("Kore Package Manager updated successfully! You can now use 'kpm' normally.");
                        tracing::info!(operation = "self_update", step = "replace_binary", "Binary replacement succeeded");
                        
                        let desktop_file = temp_dir.join("kpm.desktop");
                        let icon_file = temp_dir.join("kore-logo.svg");
                        if desktop_file.exists() {
                            let apps_dir = &config.apps_dir;
                            let icons_dir = config.apps_dir.parent().unwrap_or(&config.apps_dir).join("icons");
                            let _ = fs::create_dir_all(apps_dir);
                            let _ = fs::create_dir_all(&icons_dir);
                                
                            if let Ok(content) = fs::read_to_string(&desktop_file) {
                                let icon_path = icons_dir.join("kore-logo.svg");
                                let new_content = content.lines().map(|line| {
                                    if line.starts_with("Icon=") {
                                        format!("Icon={}", icon_path.display())
                                    } else {
                                        line.to_string()
                                    }
                                }).collect::<Vec<_>>().join("\n");
                                let _ = fs::write(apps_dir.join("kpm.desktop"), new_content);
                            } else {
                                let _ = fs::copy(&desktop_file, apps_dir.join("kpm.desktop"));
                            }
    
                            if icon_file.exists() {
                                let _ = fs::copy(&icon_file, icons_dir.join("kore-logo.svg"));
                            }
                            let _ = std::process::Command::new("update-desktop-database").arg(apps_dir).output();
                        }
                    } else {
                        error_msg("Error replacing the current binary. Make sure you have permissions.");
                        let _ = fs::remove_dir_all(&temp_dir);
                        return Err(crate::error::KoreError::Generic(
                            "Failed to replace current binary".to_string(),
                        ));
                    }
                } else {
                    error_msg("Binary 'kpm' not found inside the extracted package.");
                    let _ = fs::remove_dir_all(&temp_dir);
                    return Err(crate::error::KoreError::Generic(
                        "Extracted package does not contain 'kpm' binary".to_string(),
                    ));
                }
            } else {
                error_msg("Failed to extract the downloaded update package.");
                let _ = fs::remove_dir_all(&temp_dir);
                return Err(crate::error::KoreError::Generic(
                    "Failed to extract downloaded update package".to_string(),
                ));
            }
            let _ = fs::remove_dir_all(&temp_dir);
        }
        Err(e) => {
            error_msg("Could not download the package from GitHub.");
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(crate::error::KoreError::Generic(format!(
                "Could not download package from GitHub: {}",
                e
            )));
        }
    }

    tracing::info!(operation = "self_update", step = "finish", "Self-update finished successfully");
    Ok(())
}
