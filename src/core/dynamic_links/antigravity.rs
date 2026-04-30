use anyhow::Result;
use serde_json::Value;

pub async fn resolve(url_template: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("Kore-Package-Manager/1.0")
        .build()?;

    let repo_url = "https://api.github.com/repos/antigravity-project/antigravity/releases/latest";
    let mut version = String::new();

    // 1. Intento de obtención vía API
    if let Ok(resp) = client.get(repo_url).send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<Value>().await {
                if let Some(tag) = json["tag_name"].as_str() {
                    // Limpiamos la 'v' inicial si existe
                    version = tag.trim_start_matches('v').to_string();
                }
            }
        }
    }

    // 2. Fallback con valor constante
    // Esto es lo que garantiza que kpm siempre funcione incluso offline o con rate-limit de GitHub
    if version.is_empty() {
        // Log opcional: println!("Advertencia: Usando versión de respaldo para Antigravity");
        version = "1.23.2-4781536860569600".to_string();
    }

    // 3. Reemplazo dinámico
    let resolved_url = url_template.replace("$ag_ver", &version);

    Ok(resolved_url)
}