use std::fs;
use std::path::{Path, PathBuf};
use ratatui::widgets::ListState;
use tm::config::Config;

use tm::core::install::InstallMessage;

#[derive(PartialEq, Clone, Copy)]
pub enum Route {
    MainMenu,
    ManageApps,
    RemoveApps,
    FileBrowser,
    IconBrowser,
    ManageRepos,
    RepoCategorySelect,
}

#[derive(PartialEq, Clone, Copy)]
pub enum PopupType {
    None,
    ActionSelect,
    NameInput,
    CategorySelect,
    ChangeBinarySelect,
    ChangeRootSelect,
    ConfirmUninstall,
    InstallNameInput,
    InstallRootSelect,
    InstallCategorySelect,
    InstallBinarySelect,
    InstallProgress,
    InstallAssetSelect,
    InstallDesktopSelect,
    EnvVarInput,
    Information,
    Help,
    RepoActionSelect,
    RepoNameInput,
    RepoPackageNameInput,
    RepoUrlInput,
    RepoCategoryInput,
    RepoRootInput,
    Logs,
}

pub struct App {
    pub route: Route,
    pub apps: Vec<String>,
    pub filtered: Vec<String>,
    pub list_state: ListState,
    pub input: String,

    pub current_dir: PathBuf,
    pub fb_items: Vec<String>,
    pub fb_filtered: Vec<String>,
    pub fb_state: ListState,
    pub fb_input: String,

    pub cached_preview: Option<(String, String)>,

    pub popup_type: PopupType,
    pub popup_state: ListState,
    pub popup_items: Vec<String>,
    pub popup_input: String,
    pub popup_info: String,

    pub pending_tarball: PathBuf,
    pub pending_raw_name: String,
    pub pending_app_name: String,
    pub pending_use_root: bool,
    pub pending_category: String,
    pub pending_target: PathBuf,
    pub pending_executables: Vec<PathBuf>,
    pub pending_selected_exec: PathBuf,
    pub pending_icon_target: String,

    pub repos: Vec<tm::repo::RepoSource>,
    pub filtered_repos: Vec<tm::repo::RepoSource>,
    pub pending_repo_name: String,
    pub pending_repo_package_name: String,
    pub pending_repo_url: String,
    pub pending_repo_category: String,
    pub pending_repo_root: bool,
    pub repo_category_state: ListState,
    pub viewing_repo_type: tm::repo::RepoType,

    pub install_status: String,
    pub install_progress: f64,
    pub install_rx: Option<tokio::sync::mpsc::UnboundedReceiver<InstallMessage>>,
    pub pending_install_reply: Option<tokio::sync::oneshot::Sender<usize>>,
    pub install_done: bool,
    pub help_scroll: u16,
    pub logs_scroll: u16,
    pub logs: Vec<String>,
}

impl App {
    pub fn new(start_dir: &Path) -> Self {
        App {
            route: Route::MainMenu,
            apps: Vec::new(),
            filtered: Vec::new(),
            list_state: ListState::default(),
            input: String::new(),
            current_dir: start_dir.to_path_buf(),
            fb_items: Vec::new(),
            fb_filtered: Vec::new(),
            fb_state: ListState::default(),
            fb_input: String::new(),
            cached_preview: None,
            popup_type: PopupType::None,
            popup_state: ListState::default(),
            popup_items: Vec::new(),
            popup_input: String::new(),
            popup_info: String::new(),
            pending_tarball: PathBuf::new(),
            pending_raw_name: String::new(),
            pending_app_name: String::new(),
            pending_use_root: false,
            pending_category: String::new(),
            pending_target: PathBuf::new(),
            pending_executables: Vec::new(),
            pending_selected_exec: PathBuf::new(),
            pending_icon_target: String::new(),
            repos: Vec::new(),
            filtered_repos: Vec::new(),
            pending_repo_name: String::new(),
            pending_repo_package_name: String::new(),
            pending_repo_url: String::new(),
            pending_repo_category: String::new(),
            pending_repo_root: false,
            repo_category_state: ListState::default(),
            viewing_repo_type: tm::repo::RepoType::Official,
            install_status: String::new(),
            install_progress: 0.0,
            install_rx: None,
            pending_install_reply: None,
            install_done: false,
            help_scroll: 0,
            logs_scroll: 0,
            logs: Vec::new(),
        }
    }

    pub fn open_popup_list(&mut self, p_type: PopupType, items: Vec<String>) {
        self.popup_type = p_type;
        self.popup_items = items;
        self.popup_state.select(if self.popup_items.is_empty() { None } else { Some(0) });
    }

    pub fn open_popup_input(&mut self, p_type: PopupType, current_val: &str) {
        self.popup_type = p_type;
        self.popup_input = current_val.to_string();
    }

    pub fn open_popup_info(&mut self, text: &str) {
        self.popup_type = PopupType::Information;
        self.popup_info = text.to_string();
    }

    pub fn load_apps(&mut self, config: &Config) {
        self.apps.clear();
        if let Ok(entries) = fs::read_dir(&config.install_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    self.apps.push(entry.file_name().to_string_lossy().to_string());
                }
            }
        }
        self.apps.sort();
        self.input.clear();
        self.filter_apps();
    }

    pub fn filter_apps(&mut self) {
        if self.input.is_empty() {
            self.filtered = self.apps.clone();
        } else {
            let lower = self.input.to_lowercase();
            self.filtered = self
                .apps
                .iter()
                .filter(|i| i.to_lowercase().contains(&lower))
                .cloned()
                .collect();
        }
        self.list_state.select(if self.filtered.is_empty() { None } else { Some(0) });
    }

    pub fn load_repos(&mut self, config: &Config) {
        self.repos = tm::repo::get_all_repos(config);
        self.input.clear();
        self.filter_repos();
    }

    pub fn filter_repos(&mut self) {
        let base_filtered: Vec<_> = self.repos.iter()
            .filter(|r| r.repo_type == self.viewing_repo_type)
            .cloned()
            .collect();

        if self.input.is_empty() {
            self.filtered_repos = base_filtered;
        } else {
            let lower = self.input.to_lowercase();
            self.filtered_repos = base_filtered
                .into_iter()
                .filter(|r| r.repo.name.to_lowercase().contains(&lower) || r.repo.package_name.to_lowercase().contains(&lower) || r.repo.category.to_lowercase().contains(&lower))
                .collect();
        }
        self.list_state.select(if self.filtered_repos.is_empty() { None } else { Some(0) });
    }

    pub fn load_dir(&mut self) {
        self.fb_items.clear();
        if let Some(parent) = self.current_dir.parent() {
            if parent.to_string_lossy() != "" {
                self.fb_items.push("../".to_string());
            }
        } else {
             self.fb_items.push("../".to_string());
        }
        self.fb_items.push("./".to_string());

        if let Ok(paths) = fs::read_dir(&self.current_dir) {
            for entry in paths.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if entry.path().is_dir() {
                    self.fb_items.push(format!("{}/", name));
                } else if name.contains(".tar.") {
                    self.fb_items.push(name);
                } else if self.route == Route::IconBrowser {
                    if name.ends_with(".png") || name.ends_with(".svg") || name.ends_with(".ico") {
                        self.fb_items.push(name);
                    }
                }
            }
        }
        if self.fb_items.len() > 2 {
            let slice = &mut self.fb_items[2..];
            slice.sort();
        }
        
        self.fb_input.clear();
        self.filter_fb();
    }

    pub fn filter_fb(&mut self) {
        if self.fb_input.is_empty() {
            self.fb_filtered = self.fb_items.clone();
        } else {
            let lower = self.fb_input.to_lowercase();
            let mut filtered = Vec::new();
            if self.fb_items.len() >= 2 {
                filtered.push(self.fb_items[0].clone());
                filtered.push(self.fb_items[1].clone());
            }
            if self.fb_items.len() > 2 {
                filtered.extend(
                    self.fb_items.iter().skip(2).filter(|i| i.to_lowercase().contains(&lower)).cloned()
                );
            }
            self.fb_filtered = filtered;
        }
        self.fb_state.select(if self.fb_filtered.is_empty() { None } else { Some(0) });
    }

    pub fn next(&mut self) {
        if self.popup_type == PopupType::ActionSelect || self.popup_type == PopupType::CategorySelect
          || self.popup_type == PopupType::ChangeBinarySelect || self.popup_type == PopupType::ChangeRootSelect
          || self.popup_type == PopupType::ConfirmUninstall || self.popup_type == PopupType::InstallRootSelect 
          || self.popup_type == PopupType::InstallCategorySelect || self.popup_type == PopupType::InstallBinarySelect
          || self.popup_type == PopupType::RepoActionSelect || self.popup_type == PopupType::RepoRootInput
          || self.popup_type == PopupType::InstallAssetSelect || self.popup_type == PopupType::InstallDesktopSelect {
            let i = match self.popup_state.selected() {
                Some(i) => (i + 1) % self.popup_items.len(),
                None => 0,
            };
            self.popup_state.select(Some(i));
        } else if self.route == Route::MainMenu {
            let len = 5; // Updated for Repositories menu
            let i = match self.list_state.selected() {
                Some(i) => (i + 1) % len,
                None => 0,
            };
            self.list_state.select(Some(i));
        } else if self.route == Route::FileBrowser || self.route == Route::IconBrowser {
            let i = match self.fb_state.selected() {
                Some(i) => {
                    if i >= self.fb_filtered.len().saturating_sub(1) { 0 } else { i + 1 }
                }
                None => 0,
            };
            self.fb_state.select(Some(i));
        } else if self.route == Route::RepoCategorySelect {
            let i = match self.repo_category_state.selected() {
                Some(i) => (i + 1) % 3,
                None => 0,
            };
            self.repo_category_state.select(Some(i));
        } else if self.route == Route::ManageRepos {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i >= self.filtered_repos.len().saturating_sub(1) { 0 } else { i + 1 }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        } else {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i >= self.filtered.len().saturating_sub(1) { 0 } else { i + 1 }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }

    pub fn previous(&mut self) {
        if self.popup_type == PopupType::ActionSelect || self.popup_type == PopupType::CategorySelect
          || self.popup_type == PopupType::ChangeBinarySelect || self.popup_type == PopupType::ChangeRootSelect
          || self.popup_type == PopupType::ConfirmUninstall || self.popup_type == PopupType::InstallRootSelect 
          || self.popup_type == PopupType::InstallCategorySelect || self.popup_type == PopupType::InstallBinarySelect
          || self.popup_type == PopupType::RepoActionSelect || self.popup_type == PopupType::RepoRootInput
          || self.popup_type == PopupType::InstallAssetSelect || self.popup_type == PopupType::InstallDesktopSelect {
            let i = match self.popup_state.selected() {
                Some(i) => {
                    if i == 0 { self.popup_items.len().saturating_sub(1) } else { i - 1 }
                }
                None => 0,
            };
            self.popup_state.select(Some(i));
        } else if self.route == Route::MainMenu {
            let len = 5;
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i == 0 { len - 1 } else { i - 1 }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        } else if self.route == Route::FileBrowser || self.route == Route::IconBrowser {
            let i = match self.fb_state.selected() {
                Some(i) => {
                    if i == 0 { self.fb_filtered.len().saturating_sub(1) } else { i - 1 }
                }
                None => 0,
            };
            self.fb_state.select(Some(i));
        } else if self.route == Route::RepoCategorySelect {
            let i = match self.repo_category_state.selected() {
                Some(i) => {
                    if i == 0 { 2 } else { i - 1 }
                }
                None => 0,
            };
            self.repo_category_state.select(Some(i));
        } else if self.route == Route::ManageRepos {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i == 0 { self.filtered_repos.len().saturating_sub(1) } else { i - 1 }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        } else {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i == 0 { self.filtered.len().saturating_sub(1) } else { i - 1 }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }
}
