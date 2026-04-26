use crate::config::Config;

pub struct ResolvedSource {
    pub url: String,
    pub is_git: bool,
    pub repo_name: Option<String>,
    pub repo_package_name: Option<String>,
    pub repo_category: Option<String>,
    pub repo_requires_root: Option<bool>,
    pub repo_terminal: Option<bool>,
}

pub async fn resolve_source(config: &Config, source: &str) -> Result<Option<ResolvedSource>, crate::error::KoreError> {
    let all_repos = crate::core::repo::get_all_repos(config);
    if let Some(repo_source) = all_repos.iter().find(|r| r.repo.name.to_lowercase() == source.to_lowercase() || (!r.repo.package_name.is_empty() && r.repo.package_name.to_lowercase() == source.to_lowercase())) {
        let repo_name = Some(repo_source.repo.name.clone());
        let repo_package_name = if !repo_source.repo.package_name.is_empty() {
            Some(repo_source.repo.package_name.clone())
        } else {
            Some(repo_source.repo.name.clone())
        };
        let repo_category = Some(repo_source.repo.category.clone());
        let repo_requires_root = Some(repo_source.repo.requires_root);
        let repo_terminal = repo_source.repo.terminal;
        let url = repo_source.repo.url.clone();
        let is_git = crate::core::download::is_supported_git_url(&url);
        
        Ok(Some(ResolvedSource {
            url,
            is_git,
            repo_name,
            repo_package_name,
            repo_category,
            repo_requires_root,
            repo_terminal,
        }))
    } else {
        Ok(None)
    }
}
