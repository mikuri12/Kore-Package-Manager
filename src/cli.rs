use clap::{Parser, Subcommand};


#[derive(Parser, Debug)]
#[command(name = "tm")]
#[command(version)]
#[command(about = "TARBALL MANAGER (tm)", long_about = None)]
#[command(disable_version_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short = 'v', short_alias = 'V', long = "version", action = clap::ArgAction::Version, help = "Print version")]
    pub version: Option<bool>,

    #[arg(long = "update-bin", help = "Update the program from the latest version in the repository")]
    pub update_bin: bool,
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
        source: String,
        #[arg(default_value = "")]
        app_name: String,
        #[arg(default_value = "No")]
        use_root: String,
        #[arg(default_value = "Utility")]
        category: String,
    },
    
    /// Update installed applications from repositories
    #[command(name = "update", short_flag = 'u')]
    Update {
        #[arg(help = "Specific application to update (updates all repo apps if omitted)")]
        app_name: Option<String>,
    },
    
    /// Manage repositories
    #[command(name = "repo")]
    Repo {
        #[command(subcommand)]
        repo_command: RepoCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum RepoCommands {
    /// List all repositories
    List,
    /// Fetch latest default and community repositories from GitHub
    Sync,
    /// Add a third-party repository
    Add {
        name: String,
        package_name: String,
        url: String,
        category: String,
        #[arg(default_value_t = false)]
        requires_root: bool,
    },
    /// Remove a third-party repository
    Remove {
        name: String,
    },
}
