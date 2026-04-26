use crate::config::Config;
use crate::utils::{error_msg, info_msg, success_msg};
use dialoguer::Confirm;
use std::fs;
use std::path::PathBuf;

pub async fn update_tm(config: &Config) -> Result<(), crate::error::KoreError> {
    info_msg("Looking for the latest stable version on GitHub Releases...");

    let client = reqwest::Client::builder()
        .user_agent("Kore-Package-Manager/1.0")
        .build()?;

    let response = client.get("https://api.github.com/repos/ezequielgk/Kore-Package-Manager/releases/latest").send().await;

    if response.is_err() || !response.as_ref().unwrap().status().is_success() {
        error_msg("Error connecting to GitHub API.");
        return Ok(());
    }

    let release_data: Result<crate::core::download::Release, _> = response.unwrap().json().await;
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
        return Ok(());
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
        return Ok(());
    }

    let bin_path = config.bin_dir.join("kpm");
    let temp_dir = config.install_dir.join(".kpm_update"); // Use a dir in the same filesystem
    std::fs::create_dir_all(&temp_dir)?;

    info_msg(&format!("Downloading update..."));

    match crate::core::download::download_file(&latest_url, &temp_dir, None).await {
        Ok(downloaded_file) => {
            // Extract the tarball
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

                    // Perform the swap
                    if fs::rename(&extracted_bin, &bin_path).is_ok() || fs::copy(&extracted_bin, &bin_path).is_ok() {
                        success_msg("Kore Package Manager updated successfully! You can now use 'kpm' normally.");
                        
                        // Update desktop and icon if available
                        let desktop_file = temp_dir.join("kpm.desktop");
                        let icon_file = temp_dir.join("kore.ico");
                        if desktop_file.exists() {
                            let home_dir = std::env::var("HOME").unwrap_or_default();
                            if !home_dir.is_empty() {
                                let apps_dir = PathBuf::from(&home_dir).join(".local/share/applications");
                                let icons_dir = PathBuf::from(&home_dir).join(".local/share/icons");
                                let _ = fs::create_dir_all(&apps_dir);
                                let _ = fs::create_dir_all(&icons_dir);
                                let _ = fs::copy(&desktop_file, apps_dir.join("kpm.desktop"));
                                if icon_file.exists() {
                                    let _ = fs::copy(&icon_file, icons_dir.join("kore.ico"));
                                }
                                let _ = std::process::Command::new("update-desktop-database").arg(&apps_dir).output();
                            }
                        }
                    } else {
                        error_msg("Error replacing the current binary. Make sure you have permissions.");
                    }
                } else {
                    error_msg("Binary 'kpm' not found inside the extracted package.");
                }
            } else {
                error_msg("Failed to extract the downloaded update package.");
            }
            let _ = fs::remove_dir_all(&temp_dir);
        }
        Err(_) => {
            error_msg("Could not download the package from GitHub.");
            let _ = fs::remove_dir_all(&temp_dir);
        }
    }

    Ok(())
}
