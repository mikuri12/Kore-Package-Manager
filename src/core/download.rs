use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs::File;

use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Debug, Deserialize)]
pub struct GitlabRelease {
    pub assets: GitlabAssets,
}

#[derive(Debug, Deserialize)]
pub struct GitlabAssets {
    pub links: Vec<GitlabLink>,
    #[serde(default)]
    pub sources: Vec<GitlabSource>,
}

#[derive(Debug, Deserialize)]
pub struct GitlabSource {
    pub format: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct GitlabLink {
    pub name: String,
    pub url: String,
}

pub fn is_supported_git_url(url: &str) -> bool {
    url.contains("github.com") || url.contains("gitlab.") || url.contains("codeberg.org")
}

pub fn get_latest_release_assets(url: &str) -> Result<Vec<Asset>> {
    let mut parts: Vec<&str> = url.trim_end_matches('/').split('/').collect();
    if parts.len() < 2 {
        return Err(anyhow::anyhow!("Invalid Git URL"));
    }
    let repo = parts.pop().unwrap();
    let owner = parts.pop().unwrap();

    // Determinar el host (ej: gitlab.gnome.org o github.com)
    let host = if url.contains("github.com") {
        "github.com"
    } else if url.contains("codeberg.org") {
        "codeberg.org"
    } else if let Some(h) = url.split("://").nth(1).and_then(|s| s.split('/').next()) {
        h
    } else {
        "gitlab.com"
    };

    let client = reqwest::blocking::Client::builder()
        .user_agent("Tarball-Manager/1.0")
        .build()?;

    let valid_assets = if url.contains("gitlab.") {
        let api_url = format!("https://{}/api/v4/projects/{}%2F{}/releases", host, owner, repo);
        let response = client.get(&api_url).send()?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch GitLab release: {}", response.status()));
        }
        let mut releases: Vec<GitlabRelease> = response.json()?;
        if releases.is_empty() {
            return Err(anyhow::anyhow!("No releases found on GitLab instance {}", host));
        }
        
        let first = releases.remove(0);
        let mut assets = Vec::new();

        // Agregar links (binarios subidos manualmente)
        for link in first.assets.links {
            let n = link.name.to_lowercase();
            if n.ends_with(".tar.gz") || n.ends_with(".tar.xz") || n.ends_with(".tar.bz2") || n.ends_with(".zip") {
                assets.push(Asset {
                    name: link.name,
                    browser_download_url: link.url,
                });
            }
        }

        // Agregar sources (archivos generados automáticamente por GitLab)
        for source in first.assets.sources {
            let name = format!("{}.{}", repo, source.format);
            assets.push(Asset {
                name,
                browser_download_url: source.url,
            });
        }

        assets
    } else {
        let api_url = if url.contains("codeberg.org") {
            format!("https://codeberg.org/api/v1/repos/{}/{}/releases/latest", owner, repo)
        } else {
            format!("https://api.github.com/repos/{}/{}/releases/latest", owner, repo)
        };
        let response = client.get(&api_url).send()?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch release: {}", response.status()));
        }
        let release: Release = response.json()?;
        release.assets
            .into_iter()
            .filter(|a| {
                let n = a.name.to_lowercase();
                n.ends_with(".tar.gz") || n.ends_with(".tar.xz") || n.ends_with(".tar.bz2") || n.ends_with(".zip")
            })
            .collect()
    };

    Ok(valid_assets)
}

pub fn download_file(url: &str, dest_dir: &Path, tx: Option<std::sync::mpsc::Sender<crate::core::install::InstallMessage>>) -> Result<PathBuf> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("Tarball-Manager/1.0")
        .build()?;
        
    let mut response = client.get(url).send()?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to download file: {}", response.status()));
    }
    
    // Extract filename from URL more safely, avoiding query parameters
    let file_name = url.split('?').next().unwrap_or(url)
        .split('/').last()
        .unwrap_or("downloaded_file.tar.gz");
        
    let dest_path = dest_dir.join(file_name);
    
    let mut file = File::create(&dest_path).context("Failed to create download file")?;
    
    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut buffer = [0; 8192];
    use std::io::{Read, Write};

    while let Ok(bytes_read) = response.read(&mut buffer) {
        if bytes_read == 0 {
            break; // EOF
        }
        file.write_all(&buffer[..bytes_read]).context("Failed to write to file")?;
        downloaded += bytes_read as u64;
        
        if let Some(tx) = &tx {
            if total_size > 0 {
                let progress = (downloaded as f64 / total_size as f64) * 100.0;
                let _ = tx.send(crate::core::install::InstallMessage::Progress(format!("Downloading: {:.1}%", progress), progress));
            } else {
                let _ = tx.send(crate::core::install::InstallMessage::Progress(format!("Downloading: {} bytes", downloaded), 0.0));
            }
        }
    }
    
    Ok(dest_path)
}
