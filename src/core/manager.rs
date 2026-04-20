use crate::error::TmError;
use std::path::PathBuf;

pub trait PackageManager {
    fn install(&self, source: &str, app_name: Option<&str>, use_root: Option<bool>, category: Option<&str>) -> Result<(), TmError>;
    fn remove(&self, app_name: &str) -> Result<(), TmError>;
    fn update(&self, app_name: Option<&str>) -> Result<(), TmError>;
    fn list_installed(&self) -> Result<Vec<String>, TmError>;
}

// In the future, this can be implemented for TarballManager, AppImageManager, etc.
pub struct TarballManager {
    pub install_dir: PathBuf,
}
