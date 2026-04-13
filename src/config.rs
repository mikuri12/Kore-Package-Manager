use directories::BaseDirs;
use std::path::PathBuf;

pub struct Config {
    pub install_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub apps_dir: PathBuf,
}

impl Config {
    pub fn new() -> Self {
        let base_dirs = BaseDirs::new().expect("Could not determine home directory");
        let local_data = base_dirs.data_local_dir();

        Config {
            install_dir: local_data.join("binaries"),
            bin_dir: base_dirs.home_dir().join(".local").join("bin"),
            apps_dir: local_data.join("applications"),
        }
    }

    pub fn setup_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.install_dir)?;
        std::fs::create_dir_all(&self.bin_dir)?;
        std::fs::create_dir_all(&self.apps_dir)?;
        Ok(())
    }
}
