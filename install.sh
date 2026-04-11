#!/bin/bash

REPO_URL="https://raw.githubusercontent.com/ezequielgk/Tarball-Manager/main/tm"
BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"

echo -e "\033[0;36m󰀼 Instalando Tarball Manager...\033[0m"

if curl -sSf "$REPO_URL" -o "$BIN_DIR/tm"; then
    chmod +x "$BIN_DIR/tm"
    echo -e "\033[0;32m󰄬 Instalación completada en $BIN_DIR/tm\033[0m"
    echo -e "\033[0;33m󰀪 Asegúrate de que $BIN_DIR esté en tu PATH.\033[0m"
else
    echo -e "\033[0;31m󰅚 Error al descargar el script.\033[0m"
    exit 1
fi
