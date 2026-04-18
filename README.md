# Tarball Manager (tm)
![License](https://img.shields.io/badge/license-BSD-cyan)
![Rust](https://img.shields.io/badge/language-Rust-orange)
[![Changelog](https://img.shields.io/badge/Changelog-v1.2.2-blueviolet?logo=keepachangelog&logoColor=white)](https://github.com/ezequielgk/Tarball-Manager/blob/main/CHANGELOG.md)

Un gestor de programas minimalista y universal para Linux, rediseñado completamente en **Rust**. Está diseñado específicamente para manejar aplicaciones distribuidas en **tarballs** (.tar.gz, .tar.xz, .tar.bz2). 

Ideal para usuarios de **Void Linux**, **Arch** o cualquier sistema donde necesites instalar software pre-compilado de forma aislada, limpia y con una interfaz de terminal interactiva (TUI) elegante basada en `ratatui`.

## Características principales

* **Navegación TUI**: Explora tus archivos y carpetas con una interfaz de terminal inmersiva de alto rendimiento.
* **Interfaz CLI Híbrida**: Usa el menú interactivo o ejecuta comandos directos por terminal.
* **Instalación Inteligente**: Extrae archivos en `~/.local/share/binaries` manteniendo tu HOME limpio.
* **Gestión de Binarios**: Crea enlaces simbólicos automáticamente en `~/.local/bin`.
* **Integración con el Menú**: Genera archivos `.desktop` de forma automatizada.
* **Extracción Libre de Ruido**: Ejecuta subcomandos en segundo plano (`tar`) omitiendo salidas de terminal que puedan ensuciar la interfaz (`stdout`/`stderr`).
* **Desinstalación Atómica**: Elimina la aplicación, el enlace simbólico y el acceso directo de forma limpia.

## Instalación rápida

Puedes instalar la última versión pre-compilada directamente ejecutando:

```bash
curl -sSL https://raw.githubusercontent.com/ezequielgk/Tarball-Manager/main/install.sh | bash
```

> **Nota**: Este script descarga automáticamente la versión correcta desde *GitHub Releases*. Asegúrate de que tu carpeta `~/.local/bin` esté en tu `$PATH` del sistema.

## Uso

### Modo Interactivo (TUI)
Solo tienes que llamar a la herramienta sin argumentos para abrir la interfaz:
```bash
tm
```
* Sigue las instrucciones generadas en pantalla usando tus teclas de flechas, `ENTER` (para confirmar) y `ESC` (para regresar/salir). El flujo dinámico te permite seleccionar la app, extraer y definir el binario a linkear todo de manera guiada.

---

### Interfaz de Línea de Comandos (CLI)

Para operaciones rápidas y no interactivas, soporta los siguientes comandos definidos (`clap`):

| Comando | Alias Corto | Descripción | Ejemplo de Uso |
| :--- | :--- | :--- | :--- |
| `list` | `-l`, `list-installed`| Lista las aplicaciones instaladas actualmente desde tm. | `tm list` |
| `remove` | `-r` | Desinstala completamente una app instalada. | `tm remove discord` |
| `install` | `-i` | Instala y extrae directamente una app desde un tarball. | `tm install app.tar.gz` |
| `help` | `-h`, `--help` | Imprime las opciones de ayuda completas del programa. | `tm --help` |
| *(ninguno)* | `-V`, `--version` | Muestra la versión actual de instalación. | `tm -V` |
| `--update-bin` | Actualizara el Binario a su ultima version. | `tm --update-bin` |


#### Instalación Directa
Si no quieres usar el modo interactivo, puedes instalar pasándole los argumentos directamente (el orden es: *Ruta*, *Nombre*, *PermisosRoot*, *Categoría*):
```bash
tm install "app.tar.gz" "NombreApp" "No" "Network"
# O usando el alias:
tm -i "app.tar.gz" "NombreApp" "No" "Network"
```
* **Nombre App**: Nombre que tendrá la aplicación en el sistema.
* **Permisos Root (No/Yes)**: Define si el atajo `.desktop` utilizará `pkexec` para requerir privilegios de superusuario cada vez que se ejecute.
* **Categoría**: Categoría XDG para el menú de aplicaciones (`Utility`, `Network`, `Game`, `Development`, `Graphics`, `AudioVideo`, `System`, `Office`).

#### Desinstalación Inteligente
```bash
# Borrará la carpeta, el binario y el desktop sin necesidad de usar directorios
tm remove nombre_app
# Ej usando el sub-comando corto:
tm -r nombre_app
```

---

## Estructura de directorios

Por defecto, la herramienta aísla los archivos instalados en la estructura correcta del usuario:
- **Archivos extraídos**: `~/.local/share/binaries/[app-name]`
- **Binarios globales (Symlinks)**: `~/.local/bin/[app-name]`
- **Accesos directos (XDG Desktop)**: `~/.local/share/applications/[app-name].desktop`

## Requisitos del sistema

Al estar escrito en Rust, se ha eliminado la necesidad de utilizar dependencias externas de entorno (como `fzf` o `bash`). Los únicos requisitos en tu sistema (la inmensa mayoría vienen pre-instalados por defecto en Linux) son:

- `tar`: Utilizado de fondo para la descompresión.
- `pkexec` (Opcional): Requerido únicamente si marcas una aplicación para que solicite permisos de superusuario.
- `desktop-file-utils` (`update-desktop-database`): Sirve para notificar al sistema cuando una aplicación es "desinstalada" y refrescar el menú de aplicaciones.
