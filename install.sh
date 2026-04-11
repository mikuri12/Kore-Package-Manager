#!/bin/bash

REPO_RAW_URL="https://raw.githubusercontent.com/ezequielgk/Tarball-Manager/main/tm"
BIN_DIR="$HOME/.local/bin"
INSTALL_DIR="$HOME/.local/share/binaries"

CYAN='\033[0;36m'
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

install_tm() {
    mkdir -p "$BIN_DIR"
    echo -e "${CYAN}󰀼 Instalando Tarball Manager...${NC}"
    if curl -sSf "$REPO_RAW_URL" -o "$BIN_DIR/tm"; then
        chmod +x "$BIN_DIR/tm"
        echo -e "${GREEN}󰄬 Instalado en $BIN_DIR/tm${NC}"
        echo -e "${CYAN}󰀪 Asegúrate de tener $BIN_DIR en tu PATH.${NC}"
    else
        echo -e "${RED}󰅚 Error en la descarga.${NC}"
    fi
}

uninstall_tm() {
    echo -e "${RED}󰆴 Eliminando Tarball Manager...${NC}"
    if [ -f "$BIN_DIR/tm" ]; then
        rm "$BIN_DIR/tm"
        echo -e "${GREEN}󰄬 Binario eliminado.${NC}"
    else
        echo -e "${RED}󰅚 No se encontró el binario.${NC}"
    fi
    
    echo -e "${CYAN}¿Eliminar también todas las aplicaciones instaladas? (s/N)${NC}"
    read -p ">> " choice < /dev/tty
    if [[ "$choice" =~ ^[Ss]$ ]]; then
        rm -rf "$INSTALL_DIR"
        echo -e "${GREEN}󰄬 Aplicaciones eliminadas.${NC}"
    fi
}

clear
echo -e "${CYAN}--- TARBALL MANAGER INSTALLER ---${NC}"
echo "1) Instalar"
echo "2) Desinstalar"
echo "3) Salir"
read -p "Selecciona una opción: " opt < /dev/tty

case $opt in
    1) install_tm ;;
    2) uninstall_tm ;;
    3) exit 0 ;;
    *) echo -e "${RED}Opción no válida.${NC}" ;;
esac
