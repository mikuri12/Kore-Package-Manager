#!/bin/bash

REPO="ezequielgk/Tarball-Manager"
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

install_tm() {
    local mode=${1:-"Instalando"}
    title "$mode Tarball Manager..."
    
    mkdir -p "$BIN_DIR"
    info "Buscando la última versión estable en GitHub Releases..."
    
    local LATEST_URL=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep "browser_download_url" | grep "/tm\"" | cut -d '"' -f 4)

    if [[ -z "$LATEST_URL" ]]; then
        error "No se encontró el binario 'tm' compilado en la última Release de GitHub."
        exit 1
    fi
    
    if curl -sSL "$LATEST_URL" -o "$BIN_DIR/tm"; then
        chmod +x "$BIN_DIR/tm"
        local VERSION=$("$BIN_DIR/tm" -V 2>/dev/null | awk '{print $NF}')
        success "$mode completado (Versión: $VERSION)."
        
        if [[ "$mode" == "Instalando" ]]; then
            echo ""
            info "Asegúrate de que $BIN_DIR esté en tu PATH."
            info "Si usas Fish: fish_add_path $BIN_DIR"
        fi
    else
        error "No se pudo descargar el binario desde GitHub."
    fi
}

uninstall_tm() {
    title "Desinstalando Tarball Manager..."

    if [ -f "$BIN_DIR/tm" ]; then
        rm "$BIN_DIR/tm"
        success "Binario eliminado."
    else
        error "No se encontró el binario."
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
    echo -e "${CYAN}${BOLD}TARBALL MANAGER (tm)${NC}"
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
            if [ -f "$BIN_DIR/tm" ]; then
                install_tm "Actualizando"
            else
                error "tm no está instalado."
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
