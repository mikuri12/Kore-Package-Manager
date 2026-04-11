#!/bin/bash

REPO_RAW_URL="https://raw.githubusercontent.com/ezequielgk/Tarball-Manager/main/tm"
BIN_DIR="$HOME/.local/bin"
INSTALL_DIR="$HOME/.local/share/binaries"

CYAN='\033[0;36m'
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

msg() { echo -e "${CYAN}$1${NC}"; }
success() { echo -e "${GREEN}$1${NC}"; }
error() { echo -e "${RED}$1${NC}"; }

install_tm() {
    mkdir -p "$BIN_DIR"
    msg "󰀼 Instalando Tarball Manager..."
    if curl -sSf "$REPO_RAW_URL" -o "$BIN_DIR/tm"; then
        chmod +x "$BIN_DIR/tm"
        success "󰄬 Instalado en $BIN_DIR/tm"
        msg "󰀪 Asegúrate de tener $BIN_DIR en tu PATH."
    else
        error "󰅚 Error en la descarga."
    fi
}

uninstall_tm() {
    msg "󰆴 Eliminando Tarball Manager..."
    rm -f "$BIN_DIR/tm" && success "󰄬 Binario eliminado."
    
    read -p "¿Eliminar también todas las aplicaciones instaladas? (s/N): " choice
    [[ "$choice" =~ ^[Ss]$ ]] && rm -rf "$INSTALL_DIR" && success "󰄬 Aplicaciones eliminadas."
    success "󰄬 Desinstalación finalizada."
}

clear
echo -e "${CYAN}--- TARBALL MANAGER INSTALLER ---${NC}"
echo "1) Instalar"
echo "2) Desinstalar"
echo "3) Salir"
read -p "Selecciona una opción: " opt

case $opt in
    1) install_tm ;;
    2) uninstall_tm ;;
    3) exit 0 ;;
    *) error "Opción no válida." ;;
esac
