use directories::BaseDirs;
use std::path::PathBuf;

pub struct Config {
    pub install_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub apps_dir: PathBuf,
    pub log_dir: PathBuf,
}

impl Config {
    pub fn new() -> Self {
        let base_dirs = BaseDirs::new().expect("Could not determine home directory");
        let local_data = base_dirs.data_local_dir();
        let state_dir = base_dirs.state_dir()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| base_dirs.home_dir().join(".local").join("state"));

        Config {
            install_dir: local_data.join("binaries"),
            bin_dir: base_dirs.home_dir().join(".local").join("bin"),
            apps_dir: local_data.join("applications"),
            log_dir: state_dir.join("tm"),
        }
    }

    pub fn setup_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.install_dir)?;
        std::fs::create_dir_all(&self.bin_dir)?;
        std::fs::create_dir_all(&self.apps_dir)?;
        std::fs::create_dir_all(&self.log_dir)?;
        Ok(())
    }

    pub fn setup_logging(&self) -> anyhow::Result<tracing_appender::non_blocking::WorkerGuard> {
        let file_appender = tracing_appender::rolling::daily(&self.log_dir, "tm.log");
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
