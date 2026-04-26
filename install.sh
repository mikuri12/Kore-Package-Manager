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

setup_path() {
    title "Configurando PATH..."
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
        success "PATH configurado correctamente."
        echo -e "${YELLOW}${BOLD}⚠ IMPORTANTE:${NC} Reinicia tu terminal o ejecuta: ${CYAN}source ~/.bashrc${NC}"
    else
        info "El PATH ya estaba configurado."
    fi
}

install_completions() {
    title "Instalando autocompletados..."
    
    local RAW_URL="https://raw.githubusercontent.com/$REPO/main/assets/completions"
    
    # Bash
    if [ -f "$HOME/.bashrc" ]; then
        local bash_dir="$HOME/.local/share/bash-completion/completions"
        mkdir -p "$bash_dir"
        curl -sSL "$RAW_URL/bash/kpm" -o "$bash_dir/kpm"
        info "Autocompletado de bash instalado."
    fi

    # Zsh
    if [ -f "$HOME/.zshrc" ]; then
        local zsh_dir="$HOME/.local/share/zsh/site-functions"
        mkdir -p "$zsh_dir"
        curl -sSL "$RAW_URL/zsh/_tm" -o "$zsh_dir/_tm"
        info "Autocompletado de zsh instalado."
        
        if ! grep -q "$zsh_dir" "$HOME/.zshrc" 2>/dev/null; then
            echo -e "\n# Kore Package Manager Autocompletions\nfpath=($zsh_dir \$fpath)\nautoload -Uz compinit && compinit" >> "$HOME/.zshrc"
        fi
    fi

    # Fish
    if [ -d "$HOME/.config/fish" ]; then
        local fish_dir="$HOME/.config/fish/completions"
        mkdir -p "$fish_dir"
        curl -sSL "$RAW_URL/fish/kpm.fish" -o "$fish_dir/kpm.fish"
        info "Autocompletado de fish instalado."
    fi
    
    success "Autocompletados configurados exitosamente."
}

install_tm() {
    local mode=${1:-"Instalando"}
    title "$mode Kore Package Manager..."
    
    mkdir -p "$BIN_DIR"
    info "Buscando la última versión estable en GitHub Releases..."
    
    local LATEST_URL=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep -o 'https://github.com/[^"]*kpm-linux-x86_64\.tar\.gz' | head -n 1)

    if [[ -z "$LATEST_URL" ]]; then
        error "No se encontró el empaquetado 'kpm-linux-x86_64.tar.gz' en la última Release de GitHub."
        exit 1
    fi
    
    local tmp_dir=$(mktemp -d)
    if curl -sSL "$LATEST_URL" -o "$tmp_dir/kpm.tar.gz"; then
        tar -xzf "$tmp_dir/kpm.tar.gz" -C "$tmp_dir"
        chmod +x "$tmp_dir/kpm"
        mv "$tmp_dir/kpm" "$BIN_DIR/kpm"
        
        # Instalar accesos directos si vienen en el tar
        if [ -f "$tmp_dir/kpm.desktop" ]; then
            mkdir -p "$HOME/.local/share/applications" "$HOME/.local/share/icons"
            sed -i "s|Icon=.*|Icon=$HOME/.local/share/icons/kore.ico|g" "$tmp_dir/kpm.desktop"
            mv "$tmp_dir/kpm.desktop" "$HOME/.local/share/applications/"
            mv "$tmp_dir/kore.ico" "$HOME/.local/share/icons/" 2>/dev/null || true
            update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
        fi
        
        local VERSION=$("$BIN_DIR/kpm" -V 2>/dev/null | awk '{print $NF}')
        success "$mode completado (Versión: $VERSION)."
        
        if [[ "$mode" == "Instalando" ]]; then
            echo ""
            setup_path
            # install_completions # Opcional si ya subiste los autocompletados
        fi
        rm -rf "$tmp_dir"
    else
        error "No se pudo descargar el empaquetado desde GitHub."
    fi
}

uninstall_tm() {
    title "Desinstalando Kore Package Manager..."

    if [ -f "$BIN_DIR/kpm" ]; then
        rm "$BIN_DIR/kpm"
        rm -f "$HOME/.local/share/applications/kpm.desktop"
        rm -f "$HOME/.local/share/icons/kore.ico"
        update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
        success "Binario y accesos directos eliminados."
    else
        error "No se encontró el binario kpm."
    fi

    echo ""
    echo -ne "  ${YELLOW}⚠${NC} ¿Eliminar apps en $INSTALL_DIR? (s/n): "
    read -r resp < /dev/tty
    
    if [[ "$resp" =~ ^[sS]$ ]]; then
        rm -rf "$INSTALL_DIR"
        success "Carpeta de aplicaciones eliminada."
    fi
}

main_menu() {
    clear
    echo -e "${CYAN}${BOLD}KORE PACKAGE MANAGER (kpm)${NC}"
    echo ""
    echo "  Seleccioná una opción:"
    echo ""
    echo -e "  ${CYAN}1)${NC} Instalar"
    echo -e "  ${CYAN}2)${NC} Actualizar"
    echo -e "  ${CYAN}3)${NC} Desinstalar"
    echo -e "  ${CYAN}4)${NC} Salir"
    echo ""
    
    read -rp "  Opción [1-4]: " opcion < /dev/tty

    case "$opcion" in
        1) install_tm "Instalando" ;;
        2) 
            if [ -f "$BIN_DIR/kpm" ]; then
                install_tm "Actualizando"
            else
                error "kpm no está instalado."
            fi
            ;;
        3) uninstall_tm ;;
        4) exit 0 ;;
        *) sleep 1; main_menu ;;
    esac

    echo ""
    read -rp "  Presioná Enter para continuar..." _ < /dev/tty
    main_menu
}

main_menu