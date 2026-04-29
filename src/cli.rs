use clap::{Parser, Subcommand};


#[derive(Parser, Debug)]
#[command(name = "kpm")]
#[command(version)]
#[command(about = "KORE PACKAGE MANAGER (kpm)", long_about = None)]
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
    #[command(name = "list", about = "List all installed applications", visible_alias = "list-installed", short_flag = 'l')]
    List,
    
    #[command(name = "remove", about = "Remove one or more installed applications", short_flag = 'r')]
    Remove {
        #[arg(required = true, num_args = 1..)]
        app_names: Vec<String>,
    },
    
    #[command(name = "install", about = "Install applications from a local file, or repository", short_flag = 'i')]
    Install {
        #[arg(required = true, num_args = 1..)]
        sources: Vec<String>,
        #[arg(short, long, help = "Custom name for the application (single install only)")]
        app_name: Option<String>,
        #[arg(short, long, help = "Whether to use root/pkexec (single install only)")]
        use_root: Option<String>,
        #[arg(short, long, help = "Category for the application (single install only)")]
        category: Option<String>,
    },
    
    #[command(name = "update", about = "Update installed applications from their repositories", short_flag = 'u')]
    Update {
        #[arg(help = "Specific application to update (updates all repo apps if omitted)")]
        app_name: Option<String>,
    },
    
    #[command(name = "repo", about = "Manage repositories and package lists")]
    Repo {
        #[command(subcommand)]
        repo_command: RepoCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum RepoCommands {
    #[command(about = "List configured repositories and their package counts")]
    List,
    #[command(name = "pkg-list", about = "List all available packages from all repositories")]
    PkgList,
    #[command(name = "pkg-search", about = "Search for packages in the repositories")]
    PkgSearch {
        query: String,
    },
    #[command(about = "Sync package lists from remote repositories")]
    Sync,
    #[command(about = "Add a custom user repository")]
    Add {
        name: String,
        package_name: String,
        url: String,
        category: String,
        #[arg(long, default_value_t = false)]
        requires_root: bool,
    },
    #[command(about = "Remove a custom user repository")]
    Remove {
        name: String,
    },
}
