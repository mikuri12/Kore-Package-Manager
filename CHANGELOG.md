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
