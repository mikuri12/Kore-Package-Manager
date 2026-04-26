use clap::CommandFactory;
use clap_complete::{generate_to, shells::{Bash, Fish, Zsh}};
use std::env;
use std::fs;
use std::path::PathBuf;

include!("../cli.rs");

fn main() -> std::io::Result<()> {
    let mut cmd = Cli::command();
    let bin_name = "kpm";

    let root_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
    let completions_dir = root_dir.join("assets").join("completions");

    let bash_dir = completions_dir.join("bash");
    fs::create_dir_all(&bash_dir)?;
    generate_to(Bash, &mut cmd, bin_name, &bash_dir)?;
    println!("Generated Bash completions");

    let zsh_dir = completions_dir.join("zsh");
    fs::create_dir_all(&zsh_dir)?;
    generate_to(Zsh, &mut cmd, bin_name, &zsh_dir)?;
    println!("Generated Zsh completions");

    let fish_dir = completions_dir.join("fish");
    fs::create_dir_all(&fish_dir)?;
    generate_to(Fish, &mut cmd, bin_name, &fish_dir)?;
    println!("Generated Fish completions");

    Ok(())
}
