use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs::File;

use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct Release {
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
}

#[derive(Debug, Deserialize)]
pub struct GitlabLink {
    pub name: String,
    pub url: String,
}

pub fn is_supported_git_url(url: &str) -> bool {
    url.contains("github.com") || url.contains("gitlab.com") || url.contains("codeberg.org")
}

pub fn get_latest_release_assets(url: &str) -> Result<Vec<Asset>> {
    let mut parts: Vec<&str> = url.trim_end_matches('/').split('/').collect();
    if parts.len() < 2 {
        return Err(anyhow::anyhow!("Invalid Git URL"));
    }
    let repo = parts.pop().unwrap();
    let owner = parts.pop().unwrap();

    let client = reqwest::blocking::Client::builder()
        .user_agent("Tarball-Manager/1.0")
        .build()?;

    let valid_assets = if url.contains("gitlab.com") {
        let api_url = format!("https://gitlab.com/api/v4/projects/{}%2F{}/releases", owner, repo);
        let response = client.get(&api_url).send()?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch GitLab release: {}", response.status()));
        }
        let mut releases: Vec<GitlabRelease> = response.json()?;
        if releases.is_empty() {
            return Err(anyhow::anyhow!("No releases found on GitLab"));
        }
        releases.remove(0).assets.links
            .into_iter()
            .filter(|l| {
                let n = l.name.to_lowercase();
                n.ends_with(".tar.gz") || n.ends_with(".tar.xz") || n.ends_with(".tar.bz2")
            })
            .map(|l| Asset {
                name: l.name,
                browser_download_url: l.url,
            })
            .collect()
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
                n.ends_with(".tar.gz") || n.ends_with(".tar.xz") || n.ends_with(".tar.bz2")
            })
            .collect()
    };

    Ok(valid_assets)
}

pub fn download_file(url: &str, dest_dir: &Path) -> Result<PathBuf> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("Tarball-Manager/1.0")
        .build()?;
        
    let response = client.get(url).send()?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to download file: {}", response.status()));
    }
    
    // Extract filename from URL
    let file_name = url.split('/').last().unwrap_or("downloaded_file.tar.gz");
    let dest_path = dest_dir.join(file_name);
    
    let mut file = File::create(&dest_path).context("Failed to create download file")?;
    let content = response.bytes()?;
    std::io::copy(&mut content.as_ref(), &mut file).context("Failed to write to file")?;
    
    Ok(dest_path)
}
