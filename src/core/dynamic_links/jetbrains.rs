use anyhow::Result;

pub async fn resolve(url: &str) -> Result<String> {
    let mut resolved_url = url.to_string();
    let client = reqwest::Client::builder()
        .user_agent("Tarball-Manager/1.0")
        .build()?;
    
    let resp = client.get("https://data.services.jetbrains.com/products?code=TBA&release-type=release").send().await?;
    if resp.status().is_success() {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            if let Some(build) = json.get(0)
                .and_then(|v| v.get("releases"))
                .and_then(|v| v.get(0))
                .and_then(|v| v.get("build"))
                .and_then(|v| v.as_str()) {
                resolved_url = resolved_url.replace("$jb_ver", build);
            }
        }
    }
    
    Ok(resolved_url)
}
