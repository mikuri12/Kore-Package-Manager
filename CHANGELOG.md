[![Historial de cambios](https://img.shields.io/badge/Changelog-Español-blueviolet?logo=keepachangelog&logoColor=white)](https://github.com/ezequielgk/Tarball-Manager/blob/main/CHANGELOG_es.md)

## [2.1.7 - 2.1.8] - 2026-04-30

### TUI Batch Updates & Enhancements
- **Batch Update Interface:** Added a new "Update Applications" route to the TUI. Users can now press `u` to queue all out-of-date tracked applications for an automatic batch update, or press `Enter` to update them individually.
- **Smart Update Fix:** Fixed an issue where the update mechanism would fail to match local folder names with official repository names, causing unnecessary redownloads. The system now strictly reads the `.kpm_manifest.json` from the user-defined folder.
- **Tag Prefix Normalization:** Resolved a bug causing false positive updates by stripping `v` prefixes (e.g., `v1.2.3` vs `1.2.3`) during version comparisons against GitHub release tags.
- **Graceful Cancellation:** Pressing `Esc` during asset, binary, or desktop shortcut selection popups now immediately aborts the installation/update process instead of defaulting to the first option.
- **Dynamic Log Discovery:** Restored the `daily` logging format (`kpm.log.YYYY-MM-DD`) for better history retention, while updating the TUI's internal log viewer (`F12`) to automatically scan, sort, and open the most recent log file.

## [2.1.1 - 2.1.6] - 2026-04-29

### Version Control & Manifest System
- **State Persistence:** Introduced a `.kpm_manifest.json` file generated in each app's installation directory. This stores the app name, exact version, and the user's specific installation choices (asset, binary, and desktop file selections).
- **Version Display:** The `kpm list` command now reads from local manifests and prints the exact version of each installed package alongside its name.

### Non-Interactive Smart Updates
- **Promptless Updates:** The `kpm update` engine has been fully optimized for background execution. By extracting previously saved selections from the local manifest, the update flow completely bypasses interactive prompts, silently restoring the exact user configuration.
- **Fuzzy Asset Matching:** Anticipating version-embedded filenames (e.g., `helium-v1.0.tar.gz`), the update engine features a heuristic substitution algorithm. If the exact asset name isn't found in a new release, it intelligently swaps the local version string with the remote version string to ensure the correct architecture/package is still automatically selected.
- **Self-Update Optimization:** `kpm --update-bin` now properly checks if the current binary version matches the latest GitHub release before attempting to download, cleanly aborting if an update is unnecessary.

### Interactive TUI Improvements
- **Menu Rebranding:** Renamed the main menu option from "Install New Tarball" to "Install New Package" to better reflect the broadened file support.
- **Local AppImage Support:** The TUI File Browser now actively detects and permits the selection of `.AppImage` (and `.appimage`) files for local installation, skipping extraction and processing them natively.
- **Manual Repository Tracking:** When performing a manual installation from a local file, the TUI now prompts users if they wish to track the application for updates. If accepted, users can provide a GitHub, GitLab, or Codeberg URL, which is automatically saved to their custom repositories to enable future `kpm update` syncs.

## [2.1.0] - 2026-04-29

### First-Class AppImage Support
- **Native AppImage Integration:** The core installation engine has been upgraded to natively support `.AppImage` files alongside traditional `.tar.gz`/`.zip` archives. 
- **Transparent Local Installation:** Running `kpm install /path/to/app.AppImage` automatically detects the extension, bypasses extraction, extracts the best internal icon, and configures it locally in `~/.local/share/kpm/binaries/` with full desktop integration.
- **Dual-Format Repositories:** Introduced the `"formats"` flag in the JSON schema (`default_repos.json` and `community_repos.json`), allowing packages to declare availability for both `tarball` and `appimage` concurrently. The TUI Details panel now displays these available formats.
- **Dynamic Routing:** When a repository provides multiple release formats, `kpm` lets the user choose, seamlessly routing the downloaded asset to the correct internal processor based on its extension.

## [2.0.1] - 2026-04-28

### Stability & Reliability
- **CLI Update Flow Fixed:** Corrected async update execution in `main.rs` (`await` + error propagation), and switched update-all discovery to installed app directories for reliable matching.
- **Install/Remove Hardening:** Critical failures now propagate correctly (symlink/launcher creation, remove operations), preventing false-positive success messages during partial failures.
- **Desktop Target Matching:** Improved `.desktop` target resolution to parse `Exec`/`TryExec`/`Path` accurately, avoiding substring false positives when patching/removing shortcuts.
- **Safer Extraction & Temp Handling:** Removed path `unwrap()` points in extraction commands and introduced unique per-operation download temp directories to avoid collisions.

### Logging & TUI Diagnostics
- **Stable Log Sink:** Standardized logging to `~/.local/state/kpm/kpm.log` with reliable directory creation and structured tracing fields.
- **F12 Log Viewer Upgrade:** Internal logs popup now reads directly from `kpm.log` (tail-style behavior), instead of relying on terminal stderr output.
- **TUI Noise Suppression:** Silenced maintenance command outputs (`update-desktop-database`, `touch`) to prevent background stderr messages from breaking TUI rendering.

### Launcher Compatibility Improvements
- **Script-Aware Launchers:** When selected executables are scripts (`.py`, `.sh`, `.zsh`, `.rb`, `.pl`, `.js`), KPM now creates executable wrappers with the proper interpreter in `~/.local/bin`, improving terminal and `.desktop` launch reliability.

### Branding & Assets
- **New Logo Integration:** Updated packaging/install/update/release flows to use `kore-logo.svg` instead of `kore.ico`, including `kpm.desktop`, `install.sh`, and GitHub release workflow assets.

### Quality Gates
- **Build Hygiene:** Resolved strict Clippy issues in core paths and added targeted lint allowances in complex TUI modules without removing functionality.

## [2.0.0] - 2026-04-26

### New Identity: Kore Package Manager (kpm)
- **Total Rebranding:** Tarball Manager (tm) is now **Kore Package Manager (kpm)**. All references across the binary, CLI commands, and graphical interface have been updated.
- **System Paths:** Migrated system storage from `~/.local/share/tm/` to `~/.local/share/kpm/`.
- **Error Handling:** Completely replaced the legacy `TmError` ecosystem with `KoreError` throughout the project.

### Packaging & Distribution
- **tar.gz Releases:** GitHub releases now package the application in `kpm-linux-x86_64.tar.gz`, including the `kpm` executable, `kore-logo.svg` icon, and a pre-configured `kpm.desktop`.
- **Installer & Auto-Update:** Upgraded `install.sh` and `kpm --update-bin` to download and automatically extract the new compressed format, immediately configuring desktop shortcuts system-wide.

### Refactoring & Modularization
- **core/install.rs Decoupling:** The +860-line monolith was cleanly shattered into the `src/core/install/` sub-module, establishing dedicated files for `.desktop` manipulation, extraction, operations, repository resolution, and updates.

## [1.5.3] - 2026-04-26

### Architecture & Refactoring

  - **Modular Architecture:** Successfully divided the main 900+ line monolith file into specific, decoupled components: `core` (install/remove logic), `ui` (Ratatui), `cli` (Clap), and `config`.
  - **Unique Identity (`package_name`):** The system now utilizes the JSON `package_name` field as the absolute identifier. This dictates the folder name in `~/.local/share/binaries/` and the symlink in `~/.local/bin/`, replacing long or nonsensical auto-generated names.
  - **Community Purge:** Removed all fragile logic that automatically synchronized community repositories or used visible flags, strongly prioritizing binary stability and explicit user action.

### Interactive Installation (TUI)

  - **Explicit Flow:** The installer no longer attempts to "guess" what to download or execute. The process is now fully sequential and explicit.
  - **Manual Tarball Selection:** Added a manual selection step when multiple compressed files exist in the latest release.
  - **Silent Extraction:** Extraction processes (`tar`) now run cleanly in the background without polluting the TUI render.
  - **Unified Asset Selector:** Implemented a consolidated list combining detected executables (`[BIN]`) and bundled `.desktop` files (`[DESK]`) after extraction.
  - **Smart Desktop Parsing:** If a user selects a bundled `.desktop` file, the manager parses its `Exec=` field to automatically deduce the intended binary and accurately create the system symlink.

### XDG System Integration

  - **Dynamic Terminal Control:** Introduced a `"terminal": bool` field (default `false`) in repository JSON payloads, defining whether an app requires a terminal emulator to run.
  - **`.desktop` Patching:** By default, all installed applications force `Terminal=false` (unless specified otherwise) to prevent empty console windows from spawning when launching from app menus.
  - **Native `pkexec` Support:** If a package explicitly requires root permissions (`"requires_root": true`), the `.desktop` file automatically prepends `pkexec` to the execution command.

### Community Ecosystem

  - **Contribution Staging Area:** Created the `assets/contributions/` directory exclusively for contributors to submit multi-package JSON manifests.
  - **Rust Agnosticism:** The core codebase remains completely agnostic to the staging directory. Maintainers manually review Pull Requests and "promote" approved packages directly to the official channels.
  - **Official Consolidation:** Established `default_repos.json` and `community_repos.json` as the definitive, singular sources of truth synchronized by the binary.

### Packages & Templates

  - **JetBrains Toolbox:** Officially integrated support, guaranteeing its dynamic URL parsing and graphical execution operate perfectly under the new metadata model.
## [1.5.0] - 2026-04-21

### Features

  - **Optimized Compilation Profile:** Configured an advanced `release` profile (utilizing `opt-level=3`, `lto=true`, and `strip=true`) to generate substantially faster binaries with a significantly smaller footprint.

### Performance & Refactoring

  - **Asynchronous I/O Isolation:** Heavy core functions, such as tarball extraction and system configuration, have been migrated to `tokio::task::spawn_blocking`. This definitively resolves the progress bar "freezing" issue.
  - **Asynchronous TUI Caching:** Directory size calculations and large file previews are now processed in the background. A temporary state ("Loading preview...") is displayed, 100% eliminating micro-stuttering during navigation.
  - **Memory Allocation Cleanup:** Intensive refactoring of the rendering loop to prevent excessive string cloning (`String::clone()`) in real-time, resulting in a massive reduction in RAM consumption.
  - **Global Navigation Refactoring:** Removed over fifty lines of redundant and irrelevant code within the window manager (`handlers.rs`) by implementing a clean keyboard interceptor for generic popups.

## [1.4.4-1.4.6] - 2026-04-21

### Features

  - **Migration to Async Architecture:** The program core has been fully migrated to an asynchronous model using `Tokio` and `reqwest`. This improves stability, concurrency, and enables a smoother TUI without hangs during heavy network operations.
  - **CLI Multi-Package Support:** It is now possible to install or remove multiple applications in a single command (e.g., `tm install zen waterfox` or `tm remove discord vesktop`).
  - **Master-Detail View (Repositories):** Redesigned the repository navigation interface with a 40/60 layout. You can now see the list on the left and full details (including the new description) on the right.
  - **Application Descriptions:** Added a detailed description field for each repository in the `default_repos.json` and `community_repos.json` files.
  - **Internal Log Viewer (F12):** New integrated diagnostic tool to see what is happening "under the hood" in real-time (downloads, extractions, detailed errors). Supports independent scrolling and navigation.

### Improvements

  - **Core-to-TUI Communication:** Implementation of asynchronous bidirectional channels (`tokio::sync::mpsc` and `oneshot`) for cleaner and more professional event management.
  - **Documented Help:** Added access to logs (`F12`) in the global help menu (`?`).
  - **Metadata Priority:** In bulk CLI installations, the system now automatically prioritizes the configuration defined in official repositories over manual flags, ensuring correct installations.

## [1.4.3] - 2026-04-20

### Features

  - **Full Binary Control:** Removed automatic executable selection. The user must now always confirm which binary they wish to link, even if only one is detected in the tarball.
  - **Skip Options:** Added the ability to skip binary or `.desktop` file selection during interactive installation, allowing for a "clean" extraction without creating system links.
  - **Repository Validation:** The system now verifies that a URL is valid and accessible before allowing a new custom repository to be added, preventing future installation errors.
  - **New Official Repositories:** Included **Zen Browser**, **Stoat**, **Discord**, and **Floorp** in the default package list.

### Improvements

  - **Selection Robustness:** Improved handling of response channels (`mpsc`) to prevent unexpected popup closures during installation.

## [1.4.2] - 2026-04-20

### Features

  - **Support for Official `.desktop` Files:** `tm` now detects `.desktop` files within compressed archives. Users can choose to use the developer's official file, which is dynamically patched to ensure `Exec` and `Icon` fields point to the correct paths.
  - **Automatic Terminal App Detection:** Implemented a dynamic dependency scanner (`ldd`). If a binary lacks graphical dependencies (GTK, Qt, X11, etc.), it is automatically marked as `Terminal=true` in the shortcut.
  - **`tm` Update Confirmation:** The `--update-bin` command now shows a comparison between the current version and the version found on GitHub, requesting confirmation before downloading the new binary.

### Bug Fixes

  - **Repository Sync:** Fixed GitHub URLs in the `repo sync` command, allowing configuration files to be correctly downloaded from the official repository's `assets/` folder.

## [1.4.1] - 2026-04-20

### Features & UI Improvements

  - **Asynchronous Installations (Non-blocking):** Completely eliminated terminal freezing and suspension when installing apps. Installations now occur in a background thread.
  - **Native Progress Bar:** Integrated an interactive `Gauge` widget in the TUI that displays download percentage and extraction progress in real-time for both local and repository installations.
  - **Bidirectional Channel Architecture (`mpsc`):** New messaging system (`InstallMessage`) that allows pausing the installation to prompt the user without breaking the terminal rendering.
  - **Interactive Tarball Selection:** When a GitHub repository contains multiple files (e.g., ARM or DEB versions), the TUI now lets you select which one to download via a fluid popup menu.
  - **Binary Selection:** If multiple executables are detected upon extraction, the interface will open a menu for you to choose which one should be linked to your system.
  - **Cleaner Interface:** Simplified the "Manage Repositories" menu to "Repositories." Improved the progress widget style using *Unicode* characters and fixed high contrast for better readability.

### Bug Fixes

  - **Visual Flickering and Lag Resolved:** Internal extraction and assignment functions were set to "Silent Mode" during TUI usage, preventing them from printing plain text to `stdout` and breaking the menu layout.
  - **Navigation Fix:** Fixed a bug where arrow keys moved the background repository list instead of interacting with the tarball selection popup.
  - **Repository Cursor:** Corrected a visual detail where the repository category list showed no default selection upon first entry.

## [1.4.0] - 2026-04-20

### Bug Fixes

  - **Large Downloads:** Migrated the download engine to a streaming system. This resolves the "error decoding response body" error when downloading large files by avoiding loading the entire file into RAM.
  - **URL Normalization:** Improved filename extraction from URLs containing query parameters or complex redirects.

## [1.3.1-1.3.9] - 2026-04-19

### Improvements & Integration

  - **Icon System Refinement:** Redesigned the searcher to support hyphenated name variants (e.g., `sublime-text.png` for "Sublime Text") and increased the priority of standard paths like `/icons/`, `/share/icons/`, and `/pixmaps/`.
  - **Electron App Support:** Removed search restrictions on `/build/` and `/out/` folders, allowing apps like *Heroic Launcher* to detect their icons correctly.
  - **Shortcut Normalization:** `.desktop` files now use the repository name (e.g., "Reaper") instead of the technical package name for the menu display field.
  - **File Sanitization:** Both `.desktop` files and binaries in `~/.local/bin` now use normalized names (lowercase, no spaces) to ensure compatibility with all launchers and terminals.
  - **Automatic Refresh:** Integration of `update-desktop-database` after each installation so that icon and menu changes reflect instantly without requiring a session restart.
  - **Extended Compatibility:** Restored support for `.ico` icons and added new support for `.xpm` files.

## [1.3.0] - 2026-04-19

### Features

  - **3-Tier Repository System:** Categorization into *Official*, *Community*, and *User Custom* repositories. Official and community lists are now "Read-Only" to prevent accidental corruption.
  - **Remote Repository Sync:** New CLI command `tm repo sync` to download default app lists directly from the `main` branch on GitHub without needing to update the entire binary.
  - **Automatic App Updates:** New CLI command `tm update [app_name]` that scans installed apps and automatically downloads/reinstalls their latest versions from their respective repositories.
  - **Multi-Forge Support (GitLab & Codeberg):** The download engine can now query and interpret Release APIs from `gitlab.com` and `codeberg.org` in addition to GitHub.
  - **Direct Download Fallback:** Universal support for installing apps from any static URL. If the link is not from a known Git provider, Tarball-Manager will simply download the file directly.

### Enhancements

  - **Deep Icon Search:** Redesigned the icon search algorithm. It now scans the entire tarball without depth limits using an intelligent "scoring" system, finding hidden icons even in complex folder structures.
  - **CLI Cleanup:** Removed special font icons (Nerd Fonts) from the CLI (`tm`) standard output to maximize compatibility with simple terminals, replacing them with clean brackets (`[i]`, `[+]`, `[x]`).

## [1.2.3] - 2026-04-19

### Enhancements

  - **Dynamic Categories:** The TUI now automatically scans existing `.desktop` files to discover and display user-created custom categories alongside default options.
  - **Validation Message:** Added a visual warning `(No special characters allowed)` in text input dialogs (during installation or renaming) to prevent naming errors.

## [1.2.2] - 2026-04-18

### Features

  - **Major Architecture Refactoring:** Modularized the TUI into a component-based structure (`src/tui/`). Logic is now separated into `state.rs`, `ui.rs`, `handlers.rs`, and `mod.rs`.
  - **Custom Icon Manager:** New TUI action within "Manage Installed Apps" to manually search and select custom icons (`.png`, `.svg`, `.ico`) for installed applications.
  - **Environment Variable Injection:** Support for injecting custom environment variables (e.g., `OZONE_PLATFORM=wayland`) directly into `.desktop` files from the TUI.
  - **CLI Binary Updater:** New `--update-bin` command in the CLI to automatically update the program to the latest version from the GitHub repository.

### Technical Improvements

  - **Robust Error Handling:** Full migration to `anyhow` for standardized and detailed error reporting across the core and TUI.
  - **Professional Logging System:** Integrated `tracing` and `tracing-appender`. Logs are now written to `~/.local/state/tm/tm.log` to prevent terminal output from corrupting the TUI.
  - **Smart Messaging:** Implemented the `IS_CLI` flag to conditionally toggle between file-only logs (TUI mode) and terminal output (CLI mode).

### Bug Fixes

  - Fixed path resolution and borrow checker issues in `config.rs`.
  - Cleaned up unused imports and refined the desktop file updater to be additive (preserving existing modifiers).

## [1.2.1] - 2026-04-14

### Enhancements

  - Refactored `clap` metadata (`src/cli.rs`) to dynamically pull the version from `Cargo.toml`. This ensures CLI commands (like `tm -v`) report the latest version automatically upon core package updates.

### Bug Fixes

  - Fixed the version argument in the command line to natively support the short flag `-v` (`tm -v`), in addition to `-V` and `--version`.

## [1.2.0] - 2026-04-13 (Rust TUI Edition)

### Features

  - **Full Rust Migration:** Rewrote the entire application core from Bash to Rust, substantially improving performance and maintainability.
  - **Terminal User Interface (TUI):** Implementation of an interactive graphical terminal interface using `ratatui` and `crossterm`.
  - **Dynamic Selection Flow:** New menu system and interactive dialogs powered by `dialoguer` to facilitate binary and installation path selection without manual commands.
  - **Smart Text Wrapping:** Text and container rendering with dynamic wrapping that adapts to terminal dimensions.

### Improvements

  - **Visual Corruption Prevention:** Suppressed standard output (`stdout`) and error output (`stderr`) from external commands (like `tar`) running in the background, preventing them from cluttering the interactive UI.
  - **Automated CI/CD:** Incorporated GitHub Actions workflows to automatically compile code and generate release-ready binaries.
  - **Smart Installation Script (`install.sh`):** Redesigned the installer to automatically download the appropriate pre-compiled binary from *GitHub Releases*, ensuring a faster and cleaner setup.
  - **Code Cleanup:** Resolved multiple compilation warnings and cleaned up dependencies for a professional environment.

### Bug Fixes

  - **Version and Branch Detection:** Fixed the installation script to properly query releases and use the correct version flag (`-V`).
  - Safe handling of directories and permissions during compressed file extraction.