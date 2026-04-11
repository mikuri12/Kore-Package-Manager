#!/bin/bash

REPO_RAW_URL="https://raw.githubusercontent.com/ezequielgk/Tarball-Manager/main/tm"
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
    title "Instalando Tarball Manager..."
    
    mkdir -p "$BIN_DIR"
    info "Descargando binario desde GitHub..."
    
    if curl -sSf "$REPO_RAW_URL" -o "$BIN_DIR/tm"; then
        chmod +x "$BIN_DIR/tm"
        success "Instalación completa en $BIN_DIR/tm"
        echo ""
        info "Asegúrate de que $BIN_DIR esté en tu PATH."
        info "Si usas Fish: fish_add_path $BIN_DIR"
    else
        error "No se pudo descargar el script principal."
    fi
}

uninstall_tm() {
    title "Desinstalando Tarball Manager..."

    if [ -f "$BIN_DIR/tm" ]; then
        rm "$BIN_DIR/tm"
        success "Binario eliminado."
    else
        error "No se encontró el binario en $BIN_DIR/tm"
    fi

    echo ""
    echo -ne "  ${YELLOW}⚠${NC} ¿Eliminar también todas las apps instaladas? (s/n): "
    read -r resp < /dev/tty
    
    if [[ "$resp" =~ ^[sS]$ ]]; then
        rm -rf "$INSTALL_DIR"
        success "Carpeta de aplicaciones eliminada."
    fi
}

main_menu() {
    clear
    echo -e "${CYAN}TARBALL MANAGER (tm)${NC}"
    echo ""
    echo "  Seleccioná una opción:"
    echo ""
    echo -e "  ${CYAN}1)${NC} Instalar"
    echo -e "  ${CYAN}2)${NC} Desinstalar"
    echo -e "  ${CYAN}3)${NC} Salir"
    echo ""
    
    read -rp "  Opción [1-3]: " opcion < /dev/tty

    case "$opcion" in
        1) install_tm ;;
        2) uninstall_tm ;;
        3) echo -e "\n  Saliendo..."; exit 0 ;;
        *) echo -e "\n  ${RED}✘ Opción inválida${NC}"; sleep 1; main_menu ;;
    esac

    echo ""
    read -rp "  Presioná Enter para finalizar..." _ < /dev/tty
}

main_menu
