# Kore Package Manager (kpm)

![License](https://img.shields.io/badge/license-BSD-cyan)
![Rust](https://img.shields.io/badge/language-Rust-orange)
[![Changelog](https://img.shields.io/badge/Changelog-v2.1.0-blueviolet?logo=keepachangelog&logoColor=white)](https://github.com/ezequielgk/Kore-Package-Manager/blob/main/CHANGELOG_es.md)
[![Contributing](https://img.shields.io/badge/Contribuye-aquí-green)](https://github.com/ezequielgk/Kore-Package-Manager/blob/main/CONTRIBUTING_es.md)


Un gestor de programas minimalista y universal para Linux, rediseñado completamente en **Rust**. Está diseñado específicamente para manejar aplicaciones distribuidas en **tarballs** (.tar.gz, .tar.xz, .tar.bz2) y **AppImages** (.AppImage).

Ideal para usuarios de **Void Linux**, **Arch** o cualquier sistema donde necesites instalar software pre-compilado de forma aislada y limpia, contando con una elegante interfaz de terminal interactiva (TUI) basada en `ratatui`.

## Características Principales

  * **Navegación TUI**: Explora tus archivos y carpetas con una interfaz de terminal inmersiva y de alto rendimiento.
  * **Interfaz Híbrida CLI**: Utiliza el menú interactivo o ejecuta comandos directos a través de la terminal.
  * **Instalación Inteligente**: Extrae los archivos en `~/.local/share/binaries`, manteniendo limpio tu directorio HOME.
  * **Gestión de Binarios**: Crea automáticamente enlaces simbólicos (symlinks) en `~/.local/bin`.
  * **Integración con el Menú**: Genera automáticamente archivos de acceso directo `.desktop`.
  * **Extracción sin Ruido**: Ejecuta subcomandos en segundo plano (`tar`), omitiendo salidas de terminal que puedan ensuciar la interfaz (`stdout`/`stderr`).
  * **Desinstalación Atómica**: Elimina de forma limpia la aplicación, el enlace simbólico y el acceso directo.

## Instalación Rápida

Puedes instalar la última versión pre-compilada directamente ejecutando:

```bash
curl -sSL https://raw.githubusercontent.com/ezequielgk/Tarball-Manager/main/install.sh | bash
```

> **Nota**: Este script descarga automáticamente la versión correcta desde *GitHub Releases*. Asegúrate de que la carpeta `~/.local/bin` esté en el `$PATH` de tu sistema.

## Uso

### Modo Interactivo (TUI)

Solo necesitas llamar a la herramienta sin argumentos para abrir la interfaz:

```bash
kpm
```

  * Sigue las instrucciones en pantalla usando las teclas de flecha, `ENTER` (para confirmar) y `ESC` (para retroceder/salir). El flujo dinámico te permite seleccionar la app, extraerla y definir qué binario enlazar, todo de forma guiada.

-----

### Interfaz de Línea de Comandos (CLI)

Para operaciones rápidas no interactivas, `tm` soporta los siguientes comandos definidos (`clap`):

| Comando | Alias Corto | Descripción | Ejemplo de Uso |
| :--- | :--- | :--- | :--- |
| `list` | `-l`, `list-installed`| Lista las aplicaciones instaladas actualmente. | `kpm list` |
| `remove` | `-r` | Desinstala una o varias apps instaladas. | `kpm remove discord waterfox` |
| `install` | `-i` | Instala una o varias apps desde tarballs locales o **repositorios**. | `kpm install obsidian` |
| `update` | `-u` | Actualiza apps instaladas desde los repositorios. | `kpm update` o `kpm update obsidian` |
| `repo` | *(ninguno)* | Gestiona repositorios (oficiales, comunidad y personalizados). | `kpm repo list` |
| `help` | `-h`, `--help` | Muestra todas las opciones de ayuda del programa. | `kpm --help` |
| *(ninguno)* | `-V`, `--version` | Muestra la versión actual instalada. | `kpm -V` |
| `--update-bin` | *(ninguno)* | Actualiza el binario de Kore Package Manager a su última versión. | `kpm --update-bin` |

#### Instalación Directa (Múltiple y Repositorios)

Puedes instalar varias aplicaciones directamente escribiendo su nombre (si existen en los repositorios) o la ruta de un archivo `.tar.gz` o `.AppImage` local:

```bash
kpm install obsidian waterfox discord
# O usando el alias:
kpm -i discord
```

Si deseas instalar un archivo local específico y personalizar sus metadatos (esto aplica solo a instalaciones individuales), puedes usar las siguientes banderas:

```bash
kpm install "/path/to/app.AppImage" --app-name "NombreApp" --use-root "No" --category "Network"

kpm install "/path/to/app.tar.gz" --app-name "NombreApp" --use-root "No" --category "Network"
```

  * **--app-name (-a)**: Nombre que tendrá la aplicación en el sistema.
  * **--use-root (-u)**: Define si el acceso directo `.desktop` requerirá `pkexec` (superusuario).
  * **--category (-c)**: Categoría XDG para el menú de aplicaciones (`Utility`, `Network`, `Game`, etc).

#### Desinstalación Inteligente

Puedes borrar la carpeta, el binario y el archivo `.desktop` de una o más aplicaciones simultáneamente:

```bash
kpm remove nombre_app otra_app
# Ej. usando el alias:
kpm -r nombre_app
```

#### Gestión de Repositorios (`tm repo`)

El gestor ahora soporta repositorios para descargar e instalar apps con un solo comando.

  * `kpm repo list`: Lista la cantidad de paquetes disponibles por tipo (oficial, comunidad, usuario).
  * `kpm repo pkg-list`: Muestra la lista de todos los paquetes disponibles para instalar.
  * `kpm repo pkg-search <busqueda>`: Busca un paquete en todos los repositorios por nombre.
  * `kpm repo sync`: Sincroniza/actualiza la lista de repositorios oficiales y de la comunidad.
  * `kpm repo add <nombre> <nombre_pkg> <url> <categoria> [--requires-root]`: Añade un repositorio de terceros.
  * `kpm repo remove <nombre>`: Elimina un repositorio personalizado.

#### Autocompletado (Bash, Zsh, Fish)

Al instalar `kpm` mediante `install.sh`, los scripts de autocompletado para Bash, Zsh y Fish se configuran automáticamente de forma local en tu sistema, permitiéndote presionar `TAB` para completar comandos y banderas sin esfuerzo.

-----

## Estructura de Directorios

Por defecto, la herramienta aísla los archivos instalados en la estructura adecuada del usuario:

  - **Archivos extraídos**: `~/.local/share/binaries/[nombre-app]`
  - **Binarios globales (Symlinks)**: `~/.local/bin/[nombre-app]`
  - **Accesos directos (XDG Desktop)**: `~/.local/share/applications/[nombre-app].desktop`

## Requisitos del Sistema

Al estar escrito en Rust, se ha eliminado la necesidad de dependencias externas de entorno (como `fzf` o `bash`). Los únicos requisitos en tu sistema (la gran mayoría vienen preinstalados por defecto en Linux) son:

  - `tar`: Utilizado en segundo plano para la descompresión.
  - `pkexec` (Opcional): Requerido solo si marcas una aplicación para solicitar permisos de superusuario.
  - `desktop-file-utils` (`update-desktop-database`): Utilizado para notificar al sistema cuando una aplicación es "desinstalada" y refrescar el menú de aplicaciones.