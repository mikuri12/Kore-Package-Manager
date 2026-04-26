use serde::{Deserialize, Serialize};
use std::fs;
use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Repository {
    pub name: String,
    #[serde(default)]
    pub package_name: String,
    pub url: String,
    pub category: String,
    pub requires_root: bool,
    #[serde(default)]
    pub terminal: Option<bool>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepositoryList {
    pub repositories: Vec<Repository>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoType {
    Official,
    Community,
    User,
}

#[derive(Debug, Clone)]
pub struct RepoSource {
    pub repo_type: RepoType,
    pub repo: Repository,
}

pub fn get_official_repos(config: &Config) -> Vec<Repository> {
    if !config.official_repos_file.exists() {
        return Vec::new();
    }

    if let Ok(content) = fs::read_to_string(&config.official_repos_file) {
        if let Ok(list) = serde_json::from_str::<RepositoryList>(&content) {
            return list.repositories;
        }
    }
    Vec::new()
}

pub fn get_community_repos(config: &Config) -> Vec<Repository> {
    if !config.community_repos_file.exists() {
        return Vec::new();
    }

    if let Ok(content) = fs::read_to_string(&config.community_repos_file) {
        if let Ok(list) = serde_json::from_str::<RepositoryList>(&content) {
            return list.repositories;
        }
    }
    Vec::new()
}

pub fn get_user_repos(config: &Config) -> Vec<Repository> {
    if !config.user_repos_file.exists() {
        return Vec::new();
    }

    if let Ok(content) = fs::read_to_string(&config.user_repos_file) {
        if let Ok(list) = serde_json::from_str::<RepositoryList>(&content) {
            return list.repositories;
        }
    }
    Vec::new()
}

pub fn save_user_repos(config: &Config, repos: &[Repository]) -> Result<(), crate::error::KoreError> {
    let list = RepositoryList {
        repositories: repos.to_vec(),
    };
    let content = serde_json::to_string_pretty(&list)?;
    fs::write(&config.user_repos_file, content)?;
    Ok(())
}

pub fn get_all_repos(config: &Config) -> Vec<RepoSource> {
    let mut all = Vec::new();
    for repo in get_official_repos(config) {
        all.push(RepoSource { repo_type: RepoType::Official, repo });
    }
    for repo in get_community_repos(config) {
        all.push(RepoSource { repo_type: RepoType::Community, repo });
    }
    for repo in get_user_repos(config) {
        all.push(RepoSource { repo_type: RepoType::User, repo });
    }
    all
}

pub async fn add_user_repo(
    config: &Config,
    name: &str,
    package_name: &str,
    url: &str,
    category: &str,
    requires_root: bool,
) -> Result<(), crate::error::KoreError> {
    let mut repos = get_user_repos(config);
    // Check if it already exists
    if repos.iter().any(|r| r.name.to_lowercase() == name.to_lowercase()) {
        return Err(crate::error::KoreError::Generic("A repository with that name already exists".to_string()));
    }

    // Validate URL reachability
    let client = reqwest::Client::builder()
        .user_agent("Kore-Package-Manager/1.0")
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    
    match client.get(url).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                return Err(crate::error::KoreError::Generic(format!("The URL '{}' returned status {}. Please verify it exists.", url, resp.status())));
            }
        }
        Err(e) => {
            return Err(crate::error::KoreError::Generic(format!("Could not reach the URL '{}': {}", url, e)));
        }
    }

    repos.push(Repository {
        name: name.to_string(),
        package_name: package_name.to_string(),
        url: url.to_string(),
        category: category.to_string(),
        requires_root,
        terminal: None,
        description: None,
    });
    save_user_repos(config, &repos)?;
    Ok(())
}

pub fn remove_user_repo(config: &Config, name: &str) -> Result<bool, crate::error::KoreError> {
    let mut repos = get_user_repos(config);
    let original_len = repos.len();
    repos.retain(|r| r.name.to_lowercase() != name.to_lowercase());
    
    if repos.len() < original_len {
        save_user_repos(config, &repos)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn sync_repos(config: &Config) -> Result<(), crate::error::KoreError> {
    let client = reqwest::Client::builder()
        .user_agent("Kore-Package-Manager/1.0")
        .build()?;

    let official_url = "https://raw.githubusercontent.com/ezequielgk/Tarball-Manager/main/assets/default_repos.json";
    let community_url = "https://raw.githubusercontent.com/ezequielgk/Tarball-Manager/main/assets/community_repos.json";

    let off_resp = client.get(official_url).send().await?;
    if off_resp.status().is_success() {
        let text = off_resp.text().await?;
        // validate json
        let _: RepositoryList = serde_json::from_str(&text)?;
        std::fs::write(&config.official_repos_file, text)?;
    } else {
        return Err(crate::error::KoreError::Generic("Failed to download official repositories".to_string()));
    }

    let com_resp = client.get(community_url).send().await?;
    if com_resp.status().is_success() {
        let text = com_resp.text().await?;
        // validate json
        let _: RepositoryList = serde_json::from_str(&text)?;
        std::fs::write(&config.community_repos_file, text)?;
    } else {
        return Err(crate::error::KoreError::Generic("Failed to download community repositories".to_string()));
    }

    Ok(())
}
