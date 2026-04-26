use anyhow::Result;
use regex::Regex;

pub async fn resolve(url: &str) -> Result<String> {
    let mut resolved_url = url.to_string();
    let client = reqwest::Client::builder()
        .user_agent("Tarball-Manager/1.0")
        .build()?;
    
    let resp = client.get("https://mirrors.dotsrc.org/blender/release/").send().await?;
    if resp.status().is_success() {
        let body = resp.text().await?;
        let re_major = Regex::new(r"Blender[0-9]+\.[0-9]+")?;
        let mut majors: Vec<String> = re_major.find_iter(&body).map(|m| m.as_str().to_string()).collect();
        majors.sort_by(|a, b| {
            let a_v: Vec<u32> = a[7..].split('.').filter_map(|s| s.parse().ok()).collect();
            let b_v: Vec<u32> = b[7..].split('.').filter_map(|s| s.parse().ok()).collect();
            a_v.cmp(&b_v)
        });

        if let Some(last_major) = majors.last() {
            let url_major = format!("https://mirrors.dotsrc.org/blender/release/{}/", last_major);
            let resp_full = client.get(&url_major).send().await?;
            if resp_full.status().is_success() {
                let body_full = resp_full.text().await?;
                let re_full = Regex::new(r"blender-([0-9]+\.[0-9]+\.[0-9]+)-linux-x64\.tar\.xz")?;
                let mut full_vers: Vec<String> = re_full.captures_iter(&body_full).map(|c| c[1].to_string()).collect();
                full_vers.sort_by(|a, b| {
                    let a_v: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
                    let b_v: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
                    a_v.cmp(&b_v)
                });

                if let Some(latest_full) = full_vers.last() {
                    resolved_url = resolved_url
                        .replace("$blender_major", last_major)
                        .replace("$blender_ver", latest_full);
                }
            }
        }
    }
    
    Ok(resolved_url)
}
