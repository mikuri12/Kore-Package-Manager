#!/bin/bash

REPO="ezequielgk/Kore-Package-Manager"
BIN_DIR="$HOME/.local/bin"
INSTALL_DIR="$HOME/.local/share/binaries"

CYAN='\033[0;36m'
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
NC='\033[0m'

info()    { echo -e "${CYAN}  →${NC} $1"; }
success() { echo -e "${GREEN}  ✔${NC} $1"; }
error()   { echo -e "${RED}  ✘${NC} $1"; }
title()   { echo -e "\n${BOLD}$1${NC}"; }

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

run_privileged() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
    elif command_exists sudo; then
        sudo "$@"
    else
        error "Root privileges are required to install dependencies (sudo not found)."
        return 1
    fi
}

install_system_packages() {
    local packages=("$@")
    if [ "${#packages[@]}" -eq 0 ]; then
        return 0
    fi

    if command_exists apt-get; then
        run_privileged apt-get update && run_privileged apt-get install -y "${packages[@]}"
    elif command_exists dnf; then
        run_privileged dnf install -y "${packages[@]}"
    elif command_exists pacman; then
        run_privileged pacman -Sy --noconfirm "${packages[@]}"
    elif command_exists zypper; then
        run_privileged zypper --non-interactive install "${packages[@]}"
    else
        error "Unsupported package manager. Install these manually: ${packages[*]}"
        return 1
    fi
}

install_dependencies() {
    title "Checking system dependencies..."

    local missing=()
    command_exists curl || missing+=("curl")
    command_exists tar || missing+=("tar")
    command_exists unzip || missing+=("unzip")
    command_exists update-desktop-database || missing+=("desktop-file-utils")

    if [ "${#missing[@]}" -eq 0 ]; then
        success "All required dependencies are already installed."
        return 0
    fi

    info "Missing dependencies: ${missing[*]}"
    if install_system_packages "${missing[@]}"; then
        success "Dependencies installed successfully."
    else
        error "Could not install required dependencies."
        return 1
    fi
}

setup_path() {
    title "Configuring PATH..."
    local path_line="export PATH=\"\$PATH:$BIN_DIR\""
    local fish_path_line="fish_add_path $BIN_DIR"
    local updated=false

    local shell_files=("$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.bash_profile" "$HOME/.profile")
    
    for file in "${shell_files[@]}"; do
        if [ -f "$file" ]; then
            if ! grep -q "$BIN_DIR" "$file"; then
                echo -e "\n# Kore Package Manager\n$path_line" >> "$file"
                updated=true
            fi
        fi
    done

    if [ -d "$HOME/.config/fish" ]; then
        local fish_conf="$HOME/.config/fish/config.fish"
        mkdir -p "$(dirname "$fish_conf")"
        if ! grep -q "$BIN_DIR" "$fish_conf" 2>/dev/null; then
            echo -e "\n# Kore Package Manager\n$fish_path_line" >> "$fish_conf"
            updated=true
        fi
    fi

    if [ "$updated" = true ]; then
        success "PATH configured successfully."
        echo -e "${YELLOW}${BOLD}⚠ IMPORTANT:${NC} Restart your terminal or run: ${CYAN}source ~/.bashrc${NC}"
    else
        info "PATH is already configured."
    fi
}

install_completions() {
    title "Installing shell completions..."
    
    local RAW_URL="https://raw.githubusercontent.com/$REPO/main/assets/completions"
    
    # Bash
    if [ -f "$HOME/.bashrc" ]; then
        local bash_dir="$HOME/.local/share/bash-completion/completions"
        mkdir -p "$bash_dir"
        curl -sSL "$RAW_URL/bash/kpm" -o "$bash_dir/kpm"
        info "Bash completion installed."
    fi

    # Zsh
    if [ -f "$HOME/.zshrc" ]; then
        local zsh_dir="$HOME/.local/share/zsh/site-functions"
        mkdir -p "$zsh_dir"
        curl -sSL "$RAW_URL/zsh/_tm" -o "$zsh_dir/_tm"
        info "Zsh completion installed."
        
        if ! grep -q "$zsh_dir" "$HOME/.zshrc" 2>/dev/null; then
            echo -e "\n# Kore Package Manager Autocompletions\nfpath=($zsh_dir \$fpath)\nautoload -Uz compinit && compinit" >> "$HOME/.zshrc"
        fi
    fi

    # Fish
    if [ -d "$HOME/.config/fish" ]; then
        local fish_dir="$HOME/.config/fish/completions"
        mkdir -p "$fish_dir"
        curl -sSL "$RAW_URL/fish/kpm.fish" -o "$fish_dir/kpm.fish"
        info "Fish completion installed."
    fi
    
    success "Shell completions configured successfully."
}

install_tm() {
    local mode=${1:-"Installing"}
    title "$mode Kore Package Manager..."

    install_dependencies || exit 1

    mkdir -p "$BIN_DIR"
    info "Looking for the latest stable version on GitHub Releases..."
    
    local LATEST_URL=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep -o 'https://github.com/[^"]*kpm-linux-x86_64\.tar\.gz' | head -n 1)

    if [[ -z "$LATEST_URL" ]]; then
        error "Could not find 'kpm-linux-x86_64.tar.gz' in the latest GitHub release."
        exit 1
    fi
    
    local tmp_dir=$(mktemp -d)
    if curl -sSL "$LATEST_URL" -o "$tmp_dir/kpm.tar.gz"; then
        tar -xzf "$tmp_dir/kpm.tar.gz" -C "$tmp_dir"
        chmod +x "$tmp_dir/kpm"
        mv "$tmp_dir/kpm" "$BIN_DIR/kpm"
        
        if [ -f "$tmp_dir/kpm.desktop" ]; then
            mkdir -p "$HOME/.local/share/applications" "$HOME/.local/share/icons"
            sed -i "s|Icon=.*|Icon=$HOME/.local/share/icons/kore-logo.svg|g" "$tmp_dir/kpm.desktop"
            mv "$tmp_dir/kpm.desktop" "$HOME/.local/share/applications/"
            mv "$tmp_dir/kore-logo.svg" "$HOME/.local/share/icons/" 2>/dev/null || true
            update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
        fi
        
        local VERSION=$("$BIN_DIR/kpm" -V 2>/dev/null | awk '{print $NF}')
        success "$mode completed (Version: $VERSION)."
        
        if [[ "$mode" == "Installing" ]]; then
            echo ""
            setup_path
        fi
        rm -rf "$tmp_dir"
    else
        error "Could not download the package from GitHub."
    fi
}

uninstall_tm() {
    title "Uninstalling Kore Package Manager..."

    if [ -f "$BIN_DIR/kpm" ]; then
        rm "$BIN_DIR/kpm"
        rm -f "$HOME/.local/share/applications/kpm.desktop"
        rm -f "$HOME/.local/share/icons/kore-logo.svg"
        update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
        success "Binary and desktop entries removed."
    else
        error "kpm binary was not found."
    fi

    echo ""
    echo -ne "  ${YELLOW}⚠${NC} Remove installed apps in $INSTALL_DIR? (y/n): "
    read -r resp < /dev/tty
    
    if [[ "$resp" =~ ^[yY]$ ]]; then
        rm -rf "$INSTALL_DIR"
        success "Applications folder removed."
    fi
}

main_menu() {
    clear
    echo -e "${CYAN}${BOLD}KORE PACKAGE MANAGER (kpm)${NC}"
    echo ""
    echo "  Select an option:"
    echo ""
    echo -e "  ${CYAN}1)${NC} Install"
    echo -e "  ${CYAN}2)${NC} Update"
    echo -e "  ${CYAN}3)${NC} Uninstall"
    echo -e "  ${CYAN}4)${NC} Exit"
    echo ""
    
    read -rp "  Option [1-4]: " opcion < /dev/tty

    case "$opcion" in
        1) install_tm "Installing" ;;
        2) 
            if [ -f "$BIN_DIR/kpm" ]; then
                install_tm "Updating"
            else
                error "kpm is not installed."
            fi
            ;;
        3) uninstall_tm ;;
        4) exit 0 ;;
        *) sleep 1; main_menu ;;
    esac

    echo ""
    read -rp "  Press Enter to continue..." _ < /dev/tty
    main_menu
}

main_menu