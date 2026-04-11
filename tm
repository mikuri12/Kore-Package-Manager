#!/bin/bash

INSTALL_DIR="$HOME/.local/share/binaries"
BIN_DIR="$HOME/.local/bin"
APPS_DIR="$HOME/.local/share/applications"
mkdir -p "$INSTALL_DIR" "$BIN_DIR" "$APPS_DIR"

CYAN='\033[0;36m'
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m'

info_msg() { echo -e "${CYAN}󰋼 $1${NC}"; }
success_msg() { echo -e "${GREEN}󰄬 $1${NC}"; }
error_msg() { echo -e "${RED}󰅚 $1${NC}"; }
warn_msg() { echo -e "${YELLOW}󰀪 $1${NC}"; }

file_browser() {
    local current_dir="$1"
    while true; do
        local choice=$(ls -ap "$current_dir" | fzf \
            --height=80% --reverse --border=rounded \
            --prompt="󰉋 $current_dir > " \
            --header="ENTER: Seleccionar | ESC: Cancelar" \
            --preview "sh -c '
                p=\"$current_dir/{}\"; p=\$(echo \"\$p\" | sed \"s|//|/|g\");
                if [ -d \"\$p\" ]; then ls -F --color=always \"\$p\";
                elif echo \"\$p\" | grep -qE \"\.tar\..*\"; then tar -tf \"\$p\" 2>/dev/null | head -n 20;
                else du -sh \"\$p\" 2>/dev/null; fi'")

        [[ -z "$choice" ]] && return 1

        local full_path=$(cd "$current_dir" && pwd)/$choice
        full_path=$(echo "$full_path" | sed 's|/\./|/|g; s|//*|/|g')

        if [[ "$choice" == "../" ]]; then
            current_dir=$(dirname "$current_dir")
        elif [[ "$choice" == "./" ]]; then
            continue
        elif [[ -d "$full_path" ]]; then
            current_dir="$full_path"
        else
            echo "$full_path"
            return 0
        fi
    done
}

list_installed() {
    local APPS=$(ls -1 "$INSTALL_DIR" 2>/dev/null)
    [[ -z "$APPS" ]] && { error_msg "No hay aplicaciones instaladas."; sleep 1; return 0; }

    ls -1 "$INSTALL_DIR" | fzf \
        --height=80% --reverse --border=rounded \
        --prompt="󰏗 Apps Instaladas > " \
        --header="ESC: Volver al menú principal" \
        --preview "sh -c '
            p=\"$INSTALL_DIR/{}\";
            echo \"--- DETALLES ---\";
            echo \"Peso: \$(du -sh \"\$p\" | cut -f1)\";
            echo \"Binario: \$(ls -l \"$BIN_DIR/{}\" 2>/dev/null | awk \"{print \$NF}\")\";
            echo \"\";
            echo \"--- CONTENIDO ---\";
            ls -F --color=always \"\$p\" | head -n 15'"
    return 0
}

install_app() {
    local TARBALL=$(file_browser "$HOME")
    [[ -z "$TARBALL" ]] && return 0

    local RAW_NAME=$(basename "$TARBALL" | sed -E 's/\.tar\.(gz|xz|bz2)//')
    
    clear
    info_msg "Archivo detectado: $RAW_NAME"
    echo -e "${YELLOW}󰋼 Ingresa el nombre para el menú (Ej: Vesktop)${NC}"
    read -p ">> " APP_NAME
    [[ -z "$APP_NAME" ]] && APP_NAME="$RAW_NAME"

    local TARGET="$INSTALL_DIR/$RAW_NAME"

    if [ -d "$TARGET" ]; then
        warn_msg "La carpeta '$RAW_NAME' ya existe."
        local ACTION=$(echo -e "Cancelar\nReemplazar / Actualizar" | fzf --height=15% --reverse --border=rounded --prompt="¿Qué deseas hacer? ")
        [[ "$ACTION" != "Reemplazar / Actualizar" ]] && return 0
        
        rm -rf "$TARGET"
        rm -f "$BIN_DIR/$APP_NAME"
        rm -f "$APPS_DIR/$APP_NAME.desktop"
    fi

    mkdir -p "$TARGET"
    info_msg "Extrayendo $RAW_NAME..."
    tar -xf "$TARBALL" -C "$TARGET" --strip-components=1

    info_msg "Selecciona el binario principal"
    local EXEC_PATH=$(find "$TARGET" -maxdepth 3 -executable -type f | fzf \
        --height=40% --reverse --border=rounded --prompt="󰜎 Binario: " \
        --preview "file -b {}")

    if [[ -n "$EXEC_PATH" ]]; then
        ln -sf "$EXEC_PATH" "$BIN_DIR/$APP_NAME"
        
        local ICON_PATH=$(find "$TARGET" -maxdepth 4 \( -name "*.png" -o -name "*.svg" \) | head -n 1)
        [[ -z "$ICON_PATH" ]] && ICON_PATH="utilities-terminal"

        cat <<EOF > "$APPS_DIR/$APP_NAME.desktop"
[Desktop Entry]
Name=$APP_NAME
Exec=$BIN_DIR/$APP_NAME
Icon=$ICON_PATH
Type=Application
Terminal=false
Categories=Utility;Development;
EOF
        success_msg "¡$APP_NAME instalado! (Carpeta: $RAW_NAME)"
    else
        warn_msg "No se seleccionó binario."
    fi
    sleep 2
    return 0
}

remove_app() {
    local APPS=$(ls -1 "$INSTALL_DIR" 2>/dev/null)
    [[ -z "$APPS" ]] && { error_msg "No hay nada para eliminar."; sleep 1; return 0; }

    local TO_REMOVE=$(echo "$APPS" | fzf --height=60% --reverse --border=rounded --prompt="󰆴 Eliminar: ")
    [[ -z "$TO_REMOVE" ]] && return 0

    local CONFIRM=$(echo -e "No\nSi" | fzf --height=15% --reverse --border=rounded --prompt="¿Confirmas borrar $TO_REMOVE? ")
    if [[ "$CONFIRM" == "Si" ]]; then
        rm -rf "$INSTALL_DIR/$TO_REMOVE"
        rm -f "$BIN_DIR/$TO_REMOVE"
        rm -f "$APPS_DIR/$TO_REMOVE.desktop"
        success_msg "$TO_REMOVE eliminado."
    else
        info_msg "Operación cancelada."
    fi
    sleep 1
    return 0
}

main_menu() {
    while true; do
        clear
        local CHOICE=$(echo -e "󰉍 Instalar Nuevo Tarball\n󰏗 Gestionar Instalados\n󰆴 Desinstalar Aplicación\n󰈆 Salir" | fzf \
            --height=20% \
            --reverse \
            --border=rounded \
            --border-label=" 󰀼 TARBALL MANAGER " \
            --border-label-pos=3 \
            --prompt="󰀼 Accion > ")

        [[ -z "$CHOICE" ]] && exit 0

        case "$CHOICE" in
            *"Instalar"*) install_app ;;
            *"Gestionar"*) list_installed ;;
            *"Desinstalar"*) remove_app ;;
            *"Salir"*) exit 0 ;;
        esac
    done
}

main_menu
