use anyhow::Result;
use regex::Regex;

pub async fn resolve(url: &str) -> Result<String> {
    let mut resolved_url = url.to_string();
    let client = reqwest::Client::builder()
        .user_agent("Tarball-Manager/1.0")
        .build()?;
    
    let resp = client.get("https://cdn.waterfox.com/waterfox/releases/").send().await?;
    if resp.status().is_success() {
        let body = resp.text().await?;
        let re = Regex::new(r"G[0-9]+\.[0-9]+\.[0-9]+")?;
        
        let mut versions: Vec<String> = re.find_iter(&body)
            .map(|m| m.as_str().to_string())
            .collect();
        
        if !versions.is_empty() {
            versions.sort_by(|a, b| {
                let a_v = &a[1..];
                let b_v = &b[1..];
                let a_parts: Vec<u32> = a_v.split('.').filter_map(|s| s.parse().ok()).collect();
                let b_parts: Vec<u32> = b_v.split('.').filter_map(|s| s.parse().ok()).collect();
                a_parts.cmp(&b_parts)
            });

            if let Some(latest) = versions.last() {
                resolved_url = resolved_url.replace("$wf_ver", latest);
            }
        }
    }
    
    Ok(resolved_url)
}
