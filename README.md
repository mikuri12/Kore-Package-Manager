# Kore Package Manager (kpm)
![License](https://img.shields.io/badge/license-BSD-cyan)
![Rust](https://img.shields.io/badge/language-Rust-orange)
[![Changelog](https://img.shields.io/badge/Changelog-v2.1.0-blueviolet?logo=keepachangelog&logoColor=white)](https://github.com/ezequielgk/Kore-Package-Manager/blob/main/CHANGELOG.md)
[![Contributing](https://img.shields.io/badge/Contributing-here-green)](https://github.com/ezequielgk/Kore-Package-Manager/blob/main/CONTRIBUTING.md)
[![Readme](https://img.shields.io/badge/Readme-Español-blueviolet?logo=keepachangelog&logoColor=white)](https://github.com/ezequielgk/Kore-Package-Manager/blob/main/README_es.md)


A minimalist and universal program manager for Linux, completely redesigned in **Rust**. It is specifically designed to handle applications distributed in **tarballs** (.tar.gz, .tar.xz, .tar.bz2) and **AppImages** (.AppImage).

Ideal for users of **Void Linux**, **Arch**, or any system where you need to install pre-compiled software in an isolated, clean way, featuring an elegant interactive terminal interface (TUI) based on `ratatui`.

## Main Features

* **TUI Navigation**: Explore your files and folders with a high-performance, immersive terminal interface.
* **Hybrid CLI Interface**: Use the interactive menu or run direct commands via terminal.
* **Smart Installation**: Extracts files to `~/.local/share/binaries`, keeping your HOME directory clean.
* **Binary Management**: Automatically creates symlinks in `~/.local/bin`.
* **Menu Integration**: Automatically generates `.desktop` shortcut files.
* **Noise-Free Extraction**: Runs background subcommands (`tar`), omitting terminal outputs that could clutter the interface (`stdout`/`stderr`).
* **Atomic Uninstallation**: Cleanly removes the application, symlink, and shortcut.

## Quick Installation

You can install the latest pre-compiled version directly by running:

```bash
curl -sSL https://raw.githubusercontent.com/ezequielgk/Kore-Package-Manager/main/install.sh | bash
```

> **Note**: This script automatically downloads the correct version from *GitHub Releases*. Make sure your `~/.local/bin` folder is in your system's `$PATH`.

## Usage

### Interactive Mode (TUI)
You just need to call the tool with no arguments to open the interface:
```bash
kpm
```
* Follow the on-screen instructions using your arrow keys, `ENTER` (to confirm), and `ESC` (to go back/exit). The dynamic flow allows you to select the app, extract it, and define the binary to link—all in a guided way.

---

### Command Line Interface (CLI)

For fast, non-interactive operations, `kpm` supports the following defined commands (`clap`):

| Command | Short Alias | Description | Usage Example |
| :--- | :--- | :--- | :--- |
| `list` | `-l`, `list-installed`| Lists currently installed applications. | `kpm list` |
| `remove` | `-r` | Uninstalls one or multiple installed apps. | `kpm remove discord waterfox` |
| `install` | `-i` | Installs one or multiple apps from local tarballs or **repositories**. | `kpm install obsidian` |
| `update` | `-u` | Updates installed apps from repositories. | `kpm update` or `kpm update obsidian` |
| `repo` | *(none)* | Manages repositories (official, community, and custom). | `kpm repo list` |
| `help` | `-h`, `--help` | Prints complete help options for the program. | `kpm --help` |
| *(none)* | `-V`, `--version` | Displays the current installation version. | `kpm -V` |
| `--update-bin` | *(none)* | Updates the Kore Package Manager binary to its latest version. | `kpm --update-bin` |

#### Direct Installation (Multiple & Repositories)
You can install multiple applications directly by typing their name (if they exist in the repositories) or the path of a local `.tar.gz` or `.AppImage` file:
```bash
kpm install obsidian waterfox discord
# Or using the alias:
kpm -i discord
```
If you want to install a specific local archive and customize its metadata (this applies to single installations only), you can use the following flags:
```bash
kpm install "/path/to/app.AppImage" --app-name "NombreApp" --use-root "No" --category "Network"

kpm install "/path/to/app.tar.gz" --app-name "NombreApp" --use-root "No" --category "Network"
```
* **--app-name (-a)**: Name the application will have in the system.
* **--use-root (-u)**: Defines whether the `.desktop` shortcut will require `pkexec` (superuser).
* **--category (-c)**: XDG Category for the applications menu (`Utility`, `Network`, `Game`, etc).

#### Smart Uninstallation
You can delete the folder, binary, and `.desktop` file of one or more applications simultaneously:
```bash
kpm remove app_name another_app
# E.g. using the alias:
kpm -r app_name
```

#### Repository Management (`kpm repo`)
The manager now supports repositories to download and install apps with a single command.
* `kpm repo list`: Lists the amount of available packages by type (official, community, user).
* `kpm repo pkg-list`: Shows the list of all packages available to install.
* `kpm repo pkg-search <query>`: Searches for a package in all repositories by name.
* `kpm repo sync`: Synchronizes/updates the list of official and community repositories.
* `kpm repo add <name> <pkg_name> <url> <category> [--requires-root]`: Adds a third-party repository.
* `kpm repo remove <name>`: Removes a custom repository.

#### Shell Completions (Bash, Zsh, Fish)
When installing `kpm` via `install.sh`, autocomplete scripts for Bash, Zsh, and Fish are automatically configured locally on your system, allowing you to press `TAB` to effortlessly complete commands and flags.


## Directory Structure

By default, the tool isolates installed files into the proper user structure:
- **Extracted files**: `~/.local/share/binaries/[app-name]`
- **Global binaries (Symlinks)**: `~/.local/bin/[app-name]`
- **Shortcuts (XDG Desktop)**: `~/.local/share/applications/[app-name].desktop`

## System Requirements

Since it is written in Rust, the need for external environment dependencies (like `fzf` or `bash`) has been eliminated. The only requirements on your system (the vast majority come pre-installed by default on Linux) are:

- `tar`: Used in the background for decompression.
- `pkexec` (Optional): Required only if you mark an application to prompt for superuser permissions.
- `desktop-file-utils` (`update-desktop-database`): Used to notify the system when an application is "uninstalled" and to refresh the applications menu.
