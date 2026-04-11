# Tarball Manager (tm)
![License](https://img.shields.io/badge/license-GPL-cyan)
![Shell](https://img.shields.io/badge/shell-bash%2Fzsh%2Ffish-orange)

Un gestor de programas minimalista y universal para Linux, diseñado específicamente para manejar aplicaciones distribuidas en **tarballs** (.tar.gz, .tar.xz, .tar.bz2). 

Ideal para usuarios de **Void Linux**, **Arch** o cualquier sistema donde necesites instalar software de forma aislada, limpia y con una interfaz TUI elegante basada en `fzf`.

## Características principales

* **Navegación TUI**: Explora tus archivos y carpetas con una interfaz inspirada en Yazi/fzf.
* **Interfaz CLI Híbrida**: Usa el menú interactivo o ejecuta comandos directos por terminal.
* **Instalación Inteligente**: Extrae programas en `~/.local/share/binaries` manteniendo tu HOME limpio.
* **Gestión de Binarios**: Crea enlaces simbólicos automáticamente en `~/.local/bin`.
* **Integración con el Menú**: Genera archivos `.desktop` automáticamente con búsqueda inteligente de iconos y soporte para `pkexec`.
* **Inspección Previa**: Previsualiza el contenido de un tarball sin extraerlo.
* **Desinstalación Atómica**: Elimina la app, el binario y el acceso directo de forma limpia.

## Instalación rápida

Puedes instalarlo directamente usando `curl`:

```bash
curl -sSL https://raw.githubusercontent.com/ezequielgk/Tarball-Manager/main/install.sh | bash

```

> **Nota**: Asegúrate de tener `fzf` instalado y que `~/.local/bin` esté en tu `$PATH`.

## Uso

### Modo Interactivo (TUI)
Solo tienes que ejecutar el comando `tm` sin argumentos:
```bash
tm
```
* **ENTER**: Entrar en carpetas o seleccionar archivos/binarios.
* **ESC**: Volver atrás o cancelar la operación.
* **Filtro**: Simplemente empieza a escribir para buscar en tiempo real.

---

### Interfaz de Línea de Comandos (CLI)

Para automatizar tareas o actuar con rapidez, puedes usar los siguientes flags:

| Opción | Descripción | Ejemplo de Uso |
| :--- | :--- | :--- |
| `-l, --list` | Lista las aplicaciones instaladas. | `tm -l` |
| `-r, --remove` | Desinstala una app (soporta búsqueda parcial). | `tm -r discord` |
| `-i, --install` | Instala un tarball (directo o abre buscador). | `tm -i [args]` |
| `-v, --version` | Muestra la version instalada. | `tm -v` |
| `-h, --help` | Muestra el manual de ayuda. | `tm -h` |

#### Instalación Directa
Puedes instalar una aplicación en un solo comando pasando los parámetros requeridos:
```bash
tm -i "/ruta/archivo.tar.gz" "Nombre" "No/Si" "Categoría"
```
* **Si/No**: Define si requiere privilegios de superusuario (`pkexec`).
* **Categoría**: Categoría estándar de XDG. Las más comunes son:
    * `AudioVideo`: Reproductores de música y video.
    * `Development`: IDEs y herramientas de programación.
    * `Game`: Juegos y emuladores.
    * `Graphics`: Editores de imagen y visores.
    * `Network`: Navegadores y clientes de chat (Discord, Telegram).
    * `Office`: Herramientas de oficina y lectura.
    * `System`: Herramientas del sistema y terminales.
    * `Utility`: Utilidades generales y accesorios.
 
#### Desinstalación Inteligente
El comando de desinstalación es insensible a mayúsculas y reconoce nombres parciales:
```bash
# Borrará la carpeta aunque se llame "Vesktop-1.6.3"
tm -r vesktop
```

---

## Estructura de directorios

El script organiza los archivos de la siguiente manera:
- **Apps**: `~/.local/share/binaries/[app-name]`
- **Binarios**: `~/.local/bin/[app-name]`
- **Accesos directos**: `~/.local/share/applications/[app-name].desktop`

## Requisitos

- `bash` (para ejecutar el script)
- `fzf` (para la interfaz)
- `tar` (para la extracción)
- `Nerd Fonts` (recomendado para ver los iconos correctamente)
