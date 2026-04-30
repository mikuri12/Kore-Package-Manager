use anyhow::Result;
use serde_json::Value;

pub async fn resolve(url_template: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("Kore-Package-Manager/1.0")
        .build()?;

    let repo_url = "https://api.github.com/repos/antigravity-project/antigravity/releases/latest";
    let mut version = String::new();

    if let Ok(resp) = client.get(repo_url).send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<Value>().await {
                if let Some(tag) = json["tag_name"].as_str() {
                    version = tag.trim_start_matches('v').to_string();
                }
            }
        }
    }

    if version.is_empty() {
        version = "1.23.2-4781536860569600".to_string();
    }

    let resolved_url = url_template.replace("$ag_ver", &version);

    Ok(resolved_url)
}