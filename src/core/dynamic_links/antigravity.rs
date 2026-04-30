use anyhow::Result;
use serde_json::Value;

pub async fn resolve(url_template: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("Kore-Package-Manager/1.0")
        .build()?;

    // 1. Intentar obtener la versión desde la API de GitHub
    // Antigravity usa un WAF agresivo en su web, pero GitHub es amigable para APIs.
    let repo_url = "https://api.github.com/repos/antigravity-project/antigravity/releases/latest";
    
    let mut version = String::new();

    if let Ok(resp) = client.get(repo_url).send().await {
        if resp.status().is_success() {
            let json: Value = resp.json().await?;
            // Extraemos el tag_name (ej: "v1.23.2-4781536860569600") y limpiamos la 'v'
            if let Some(tag) = json["tag_name"].as_str() {
                version = tag.trim_start_matches('v').to_string();
            }
        }
    }

    // 2. Fallback: Si la API falla, usamos tu último valor conocido
    // Esto evita que kpm se rompa si no hay internet o GitHub cae.
    if version.is_empty() {
        version = "1.23.2-4781536860569600".to_string();
    }

    // 3. Construir la URL final
    // Reemplazamos el placeholder que definas en tu repo.json (ej: $ag_ver)
    let resolved_url = url_template.replace("$ag_ver", &version);

    Ok(resolved_url)
}
