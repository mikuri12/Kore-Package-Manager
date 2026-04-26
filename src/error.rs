use thiserror::Error;

#[derive(Error, Debug)]
pub enum KoreError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Repository not found: {0}")]
    RepoNotFound(String),
    
    #[error("Application not found: {0}")]
    AppNotFound(String),
    
    #[error("Missing executable binary in the extracted archive")]
    MissingExecutable,
    
    #[error("Extraction failed: {0}")]
    ExtractionError(String),
    
    #[error("Generic error: {0}")]
    Generic(String),
}
