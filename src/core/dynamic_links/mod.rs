pub mod tor;
pub mod sublime;
pub mod waterfox;
pub mod jetbrains;
pub mod blender;

use anyhow::Result;

pub async fn resolve_dynamic_url(url: &str) -> Result<String> {
    let mut resolved_url = url.to_string();

    if resolved_url.contains("$tor_ver") {
        resolved_url = tor::resolve(&resolved_url).await?;
    }
    if resolved_url.contains("$st_build") {
        resolved_url = sublime::resolve(&resolved_url).await?;
    }
    if resolved_url.contains("$wf_ver") {
        resolved_url = waterfox::resolve(&resolved_url).await?;
    }
    if resolved_url.contains("$jb_ver") {
        resolved_url = jetbrains::resolve(&resolved_url).await?;
    }
    if resolved_url.contains("$blender_ver") || resolved_url.contains("$blender_major") {
        resolved_url = blender::resolve(&resolved_url).await?;
    }

    Ok(resolved_url)
}
