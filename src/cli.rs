use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "tm")]
#[command(version = "1.2.0")]
#[command(about = "TARBALL MANAGER (tm)", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List installed applications
    #[command(name = "list", visible_alias = "list-installed", short_flag = 'l')]
    List,
    
    /// Uninstall app (Ex: tm remove discord)
    #[command(name = "remove", short_flag = 'r')]
    Remove {
        app_name: String,
    },
    
    /// Install application from a specific tarball
    #[command(name = "install", short_flag = 'i')]
    Install {
        tarball: PathBuf,
        #[arg(default_value = "")]
        app_name: String,
        #[arg(default_value = "No")]
        use_root: String,
        #[arg(default_value = "Utility")]
        category: String,
    },
}
