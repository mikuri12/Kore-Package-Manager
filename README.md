# Tarball Manager (tm)
![License](https://img.shields.io/badge/license-GPL-cyan)
![Shell](https://img.shields.io/badge/shell-bash%2Fzsh%2Ffish-orange)

Un gestor de programas minimalista y universal para Linux, diseñado específicamente para manejar aplicaciones distribuidas en **tarballs** (.tar.gz, .tar.xz, .tar.bz2). 

Ideal para usuarios de **Void Linux**, **Arch** o cualquier sistema donde necesites instalar software de forma aislada, limpia y con una interfaz TUI elegante basada en `fzf`.

## Características principales

* **Navegación TUI**: Explora tus archivos y carpetas con una interfaz inspirada en Yazi/fzf.
* **Instalación Inteligente**: Extrae programas en `~/.local/share/binaries` manteniendo tu HOME limpio.
* **Gestión de Binarios**: Crea enlaces simbólicos automáticamente en `~/.local/bin`.
* **Integración con el Menú**: Genera archivos `.desktop` automáticamente con búsqueda inteligente de iconos.
* **Inspección Previa**: Previsualiza el contenido de un tarball sin extraerlo.
* **Desinstalación Atómica**: Elimina la app, el binario y el acceso directo de un solo golpe.
* **Universal**: Compatible con `bash`, `zsh` y `fish`.

## Instalación rápida

Puedes instalarlo directamente usando `curl`:

```bash
curl -sSL https://raw.githubusercontent.com/ezequielgk/Tarball-Manager/main/install.sh | bash
```

> **Nota**: Asegúrate de tener `fzf` instalado y que `~/.local/bin` esté en tu `$PATH`.

## Uso

Solo tienes que ejecutar el comando `tm` en tu terminal:

```bash
tm
```

### Atajos en los menús:
- **ENTER**: Entrar en carpetas o seleccionar archivos/binarios.
- **ESC**: Volver atrás o cancelar la operación.
- **Filtro**: Simplemente empieza a escribir en cualquier menú para buscar.

## Estructura de directorios

El script organiza todo de la siguiente manera:
- **Apps**: `~/.local/share/binaries/[app-name]`
- **Binarios**: `~/.local/bin/[app-name]`
- **Accesos directos**: `~/.local/share/applications/[app-name].desktop`

## Requisitos

- `bash` (para ejecutar el script)
- `fzf` (para la interfaz TUI)
- `tar` (para la extracción)
- `Nerd Fonts` (recomendado para ver los iconos correctamente)

---
Desarrollado para entornos minimalistas.
