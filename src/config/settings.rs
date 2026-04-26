use directories::BaseDirs;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Config {
    pub install_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub apps_dir: PathBuf,
    pub log_dir: PathBuf,
    pub community_repos_file: PathBuf,
    pub user_repos_file: PathBuf,
    pub official_repos_file: PathBuf,
}

impl Config {
    pub fn new() -> Self {
        let base_dirs = BaseDirs::new().expect("Could not determine home directory");
        let local_data = base_dirs.data_local_dir();
        let state_dir = base_dirs.state_dir()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| base_dirs.home_dir().join(".local").join("state"));

        let kpm_data = local_data.join("kpm");
        Config {
            install_dir: local_data.join("binaries"),
            bin_dir: base_dirs.home_dir().join(".local").join("bin"),
            apps_dir: local_data.join("applications"),
            log_dir: state_dir.join("kpm"),
            community_repos_file: kpm_data.join("community_repos.json"),
            user_repos_file: kpm_data.join("user_repos.json"),
            official_repos_file: kpm_data.join("official_repos.json"),
        }
    }

    pub fn setup_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.install_dir)?;
        std::fs::create_dir_all(&self.bin_dir)?;
        std::fs::create_dir_all(&self.apps_dir)?;
        std::fs::create_dir_all(&self.log_dir)?;
        if let Some(parent) = self.community_repos_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Initialize official repos to sync with bundled defaults only if missing
        if !self.official_repos_file.exists() {
            let default_json = include_str!("../../assets/default_repos.json");
            std::fs::write(&self.official_repos_file, default_json)?;
        }

        // Initialize community repos to sync with bundled defaults only if missing
        if !self.community_repos_file.exists() {
            let community_json = include_str!("../../assets/community_repos.json");
            std::fs::write(&self.community_repos_file, community_json)?;
        }

        // Initialize empty user repos if file doesn't exist
        if !self.user_repos_file.exists() {
            let empty_json = "{\n  \"repositories\": []\n}";
            std::fs::write(&self.user_repos_file, empty_json)?;
        }

        Ok(())
    }

    pub fn setup_logging(&self) -> anyhow::Result<tracing_appender::non_blocking::WorkerGuard> {
        let file_appender = tracing_appender::rolling::daily(&self.log_dir, "kpm.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let subscriber = tracing_subscriber::fmt::Subscriber::builder()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(false)
            .finish();
            
        tracing::subscriber::set_global_default(subscriber)
            .map_err(|e| anyhow::anyhow!("Failed to set global tracing subscriber: {}", e))?;
            
        Ok(guard)
    }
}
