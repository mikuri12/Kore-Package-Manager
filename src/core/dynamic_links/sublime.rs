use anyhow::Result;
use regex::Regex;

pub async fn resolve(url: &str) -> Result<String> {
    let mut resolved_url = url.to_string();
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .build()?;
    
    let resp = client.get("https://www.sublimetext.com/download").send().await?;
    if resp.status().is_success() {
        let body = resp.text().await?;
        let re = Regex::new(r"sublime_text_build_([0-9]{4})_x64.tar.xz")?;
        if let Some(caps) = re.captures(&body) {
            let build = &caps[1];
            resolved_url = resolved_url.replace("$st_build", build);
        }
    }
    
    Ok(resolved_url)
}
