## [1.3.0] - 2026-04-19

### Características (Features)
- **Sistema de Repositorios de 3 Niveles:** Clasificación en repositorios *Official*, *Community* y *User Custom*. Los oficiales y comunitarios ahora están protegidos en modo "Solo Lectura", asegurando que las listas base no puedan romperse accidentalmente.
- **Sincronización Remota de Repositorios:** Nuevo comando CLI `tm repo sync` para descargar las listas de aplicaciones predeterminadas directamente desde la rama `main` del proyecto en GitHub, sin necesidad de actualizar el binario completo.
- **Actualización Automática de Apps:** Nuevo comando CLI `tm update [app_name]` que escanea tus aplicaciones instaladas y descarga/reinstala automáticamente sus últimas versiones desde sus respectivos repositorios.
- **Soporte Multi-Forja (GitLab & Codeberg):** El motor de descarga ahora es capaz de consultar e interpretar las APIs de lanzamientos (Releases) de `gitlab.com` y `codeberg.org`, además de GitHub.
- **Direct Download Fallback:** Soporte universal para instalar aplicaciones desde cualquier URL estática de internet. Si el enlace no proviene de un proveedor Git conocido, Tarball-Manager simplemente descargará el archivo directamente.

### Mejoras (Enhancements)
- **Búsqueda Profunda de Íconos:** Se rediseñó el algoritmo de búsqueda de íconos. Ahora escanea todo el tarball sin límite de profundidad utilizando un sistema inteligente de "puntuación", logrando encontrar los íconos ocultos incluso en las estructuras de carpetas más complejas.
- **Limpieza de CLI:** Se eliminaron los íconos de fuentes especiales (Nerd Fonts) de la salida estándar del CLI (`tm`) para maximizar la compatibilidad con terminales simples, reemplazándolos por corchetes limpios (`[i]`, `[+]`, `[x]`).

## [1.2.3] - 2026-04-19

### Mejoras (Enhancements)
- **Categorías Dinámicas:** La TUI ahora escanea automáticamente los archivos `.desktop` existentes para descubrir y mostrar categorías personalizadas creadas por el usuario, además de las opciones predeterminadas.
- **Mensaje de Validación:** Se agregó un mensaje de advertencia visual `(No special characters allowed)` en los diálogos de entrada de texto (al instalar o renombrar) para prevenir errores al nombrar las aplicaciones.

## [1.2.2] - 2026-04-18

### Características (Features)
- **Refactorización Mayor de la Arquitectura:** Se modularizó la TUI en una estructura basada en componentes (`src/tui/`). La lógica ahora está separada en `state.rs`, `ui.rs`, `handlers.rs` y `mod.rs`.
- **Gestor de Íconos Personalizados:** Nueva acción en la TUI dentro de "Manage Installed Apps" para buscar y seleccionar manualmente íconos personalizados (`.png`, `.svg`, `.ico`) para las aplicaciones instaladas.
- **Inyección de Variables de Entorno:** Soporte para inyectar variables de entorno personalizadas (ej: `OZONE_PLATFORM=wayland`) directamente en los archivos `.desktop` desde la TUI.
- **CLI para Actualizar Binario:** Nuevo comando `--update-bin` en la CLI para actualizar automáticamente el programa a la última versión desde el repositorio de GitHub.

### Mejoras Técnicas (Technical Improvements)
- **Manejo de Errores Robusto:** Migración completa a `anyhow` para un reporte de errores estandarizado y detallado en todo el core y la TUI.
- **Sistema de Logging Profesional:** Integración de `tracing` y `tracing-appender`. Los logs ahora se escriben en `~/.local/state/tm/tm.log` para evitar que la salida de la terminal corrompa la TUI.
- **Mensajería Inteligente:** Implementación del flag `IS_CLI` para alternar condicionalmente entre logs solo en archivo (modo TUI) y salida por terminal (modo CLI).

### Correcciones (Bug Fixes)
- Se corrigieron problemas de resolución de rutas y del "borrow checker" en `config.rs`.
- Limpieza de importaciones no utilizadas y refinamiento del actualizador de archivos desktop para que sea aditivo (preservando modificadores existentes).

## [1.2.1] - 2026-04-14

### Mejoras (Enhancements)
- Se refactorizó la metadata de `clap` (`src/cli.rs`) para que tome dinámicamente la versión desde `Cargo.toml`. De este modo, al actualizar el paquete principal, los comandos de la CLI (como `tm -v`) reportan automáticamente la última versión sin depender de valores codificados internamente.

### Correcciones (Bug Fixes)
- Se corrigió el argumento de versión en la línea de comandos para que soporte la bandera corta `-v` de forma nativa (`tm -v`), además de `-V` y `--version`.

## [1.2.0] - 2026-04-13 (Rust TUI Edition)

### Features
- **Migración Completa a Rust:** Se reescribió todo el core de la aplicación de Bash a Rust, mejorando sustancialmente el rendimiento y la mantenibilidad.
- **Terminal User Interface (TUI):** Implementación de una interfaz gráfica de terminal interactiva utilizando `ratatui` y `crossterm`.
- **Flujo de Selección Dinámico:** Nuevo sistema de menús y diálogos interactivos impulsados por `dialoguer` para facilitar la selección de binarios y rutas de instalación sin escribir comandos manuales.
- **Ajuste de Texto Inteligente:** Renderizado de texto y contenedores con "text-wrapping" que se adaptan dinámicamente a las dimensiones de la terminal.

### Improvements (Mejoras)
- **Bloqueo de Corrupción Visual:** Se suprimió la salida estándar (`stdout`) y de error (`stderr`) de los comandos externos (como `tar`) ejecutados en segundo plano, evitando que ensuciaran la interfaz interactiva.
- **CI/CD Automatizado:** Incorporación de flujos de trabajo de GitHub Actions para compilar automáticamente el código y generar binarios listos para los releases.
- **Script de Instalación Inteligente (`install.sh`):** Se rediseñó el instalador para descargar automáticamente el binario precompilado adecuado desde *GitHub Releases*, acelerando y garantizando una instalación más limpia.
- **Limpieza del Código:** Resolución de múltiples *warnings* de compilación y limpieza de dependencias garantizando un entorno profesional.

### Bug Fixes
- **Detección de Versiones y Ramas:** Arreglo en el script de instalación para consultar adecuadamente los releases y usar el flag de versión correcto (`-V`).
- Manejo seguro de directorios y permisos durante la extracción de archivos comprimidos.
