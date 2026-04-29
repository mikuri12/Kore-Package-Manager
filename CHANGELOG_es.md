## [2.1.0] - 2026-04-29

### Soporte Nativo para AppImage
- **Integración de Primera Clase:** El motor de instalación ha sido actualizado para soportar nativamente archivos `.AppImage` a la par de los tradicionales archivos `.tar.gz`/`.zip`.
- **Instalación Local Transparente:** Ejecutar `kpm install /ruta/a/la/app.AppImage` detecta automáticamente la extensión, omite la descompresión, extrae el mejor ícono interno y lo configura en `~/.local/share/kpm/binaries/` con integración completa en el sistema.
- **Repositorios de Formato Dual:** Se introdujo la bandera `"formats"` en el esquema JSON (`default_repos.json` y `community_repos.json`), permitiendo que los paquetes declaren disponibilidad tanto para `tarball` como para `appimage`. El panel de Detalles de la TUI ahora muestra los formatos disponibles.
- **Enrutamiento Dinámico:** Cuando un repositorio proporciona múltiples formatos en sus *Releases*, `kpm` permite al usuario elegir y enruta de forma fluida la descarga al procesador correcto basándose en la extensión del archivo.

## [2.0.1] - 2026-04-28
### Estabilidad y Confiabilidad
- **Flujo de Update CLI Corregido:** Se arregló la ejecución asíncrona de actualización en `main.rs` (`await` + propagación de errores), y se cambió el escaneo de update-all a directorios realmente instalados.
- **Fortalecimiento de Install/Remove:** Los fallos críticos ahora se propagan correctamente (creación de symlink/launcher y operaciones de desinstalación), evitando mensajes de éxito falsos en fallos parciales.
- **Matching Preciso de `.desktop`:** Se mejoró la resolución de accesos directos analizando `Exec`/`TryExec`/`Path`, evitando falsos positivos por coincidencias de substring al parchear o eliminar.
- **Extracción y Temporales Seguros:** Se eliminaron `unwrap()` de rutas en comandos de extracción y se agregaron directorios temporales únicos por operación para evitar colisiones.

### Logging y Diagnóstico en TUI
- **Destino de Logs Estable:** Logging estandarizado en `~/.local/state/kpm/kpm.log` con creación confiable de directorios y campos estructurados en tracing.
- **Mejora de Visor F12:** El popup de logs internos ahora lee directamente desde `kpm.log` (estilo tail), en vez de depender de stderr de la terminal.
- **Supresión de Ruido en TUI:** Se silenció la salida de comandos de mantenimiento (`update-desktop-database`, `touch`) para evitar mensajes de fondo que rompían el render de la TUI.

### Compatibilidad de Lanzadores
- **Launchers para Scripts:** Si el ejecutable seleccionado es script (`.py`, `.sh`, `.zsh`, `.rb`, `.pl`, `.js`), KPM ahora genera wrappers ejecutables con intérprete correcto en `~/.local/bin`, mejorando la ejecución desde terminal y `.desktop`.

### Branding y Assets
- **Integración del Nuevo Logo:** Se actualizó empaquetado/instalación/auto-update/release para usar `kore-logo.svg` en lugar de `kore.ico`, incluyendo `kpm.desktop`, `install.sh` y el workflow de release.

### Calidad del Proyecto
- **Higiene de Build:** Se corrigieron problemas reportados por Clippy estricto en rutas core y se añadieron `allow` puntuales en módulos TUI complejos sin eliminar funcionalidades.

## [2.0.0] - 2026-04-26

### Nueva Identidad: Kore Package Manager (kpm)
- **Rebranding Total:** Tarball Manager (tm) ahora es **Kore Package Manager (kpm)**. Se renombraron todas las referencias en el binario, los comandos CLI y la interfaz gráfica.
- **Rutas del Sistema:** Se migró el almacenamiento del sistema de `~/.local/share/tm/` a `~/.local/share/kpm/`.
- **Manejo de Errores:** Se reemplazó el antiguo ecosistema de `TmError` por `KoreError` en todo el proyecto.

### Empaquetado y Distribución
- **Releases en tar.gz:** Los lanzamientos de GitHub ahora empaquetan la app en `kpm-linux-x86_64.tar.gz`, incluyendo el binario `kpm`, el ícono `kore-logo.svg` y un `kpm.desktop` configurado.
- **Instalador y Auto-Update:** Se actualizó `install.sh` y el comando `kpm --update-bin` para descargar y extraer automáticamente el nuevo formato comprimido, configurando los accesos directos de escritorio al vuelo.

### Refactorización y Modularización
- **Desacople de core/install.rs:** El monolito de +860 líneas se fragmentó limpiamente en el sub-módulo `src/core/install/`, creando archivos dedicados para la modificación de archivos `.desktop`, extracción, operaciones, resolución de repositorios y actualizaciones.

## [1.5.3] - 2026-04-26

### Arquitectura y Organización

  - **Modularización del Proyecto:** Se planeó la división del archivo principal de 900+ líneas en componentes específicos: core (instalación/borrado), ui (Ratatui), cli (Clap) y config.
  - **Identidad Única (`package_name`):** El sistema ahora utiliza el `package_name` del JSON como identificador absoluto. Esto define el nombre de la carpeta en `~/.local/share/binaries/` y el symlink en `~/.local/bin/`, eliminando nombres de archivos largos o absurdos.
  - **Purga de Comunidad Automática:** Se eliminó toda lógica que sincronizaba repositorios de comunidad de forma automática o mediante flags visibles, priorizando la estabilidad del binario.

### Sistema de Instalación Inteligente (TUI)

  - **Eliminación de Predicciones:** El instalador ya no intenta "adivinar" qué descargar o qué ejecutar. Ahora es un proceso secuencial y explícito.
  - **Selección de Tarball:** Paso manual para elegir el archivo comprimido si hay varios en el repositorio.
  - **Extracción Silenciosa:** Los procesos de `tar` corren en segundo plano sin ensuciar la interfaz.
  - **Selector Unificado de Archivos:** Se implementó una lista que mezcla ejecutables (`[BIN]`) y archivos `.desktop` existentes (`[DESK]`) encontrados tras la extracción.
  - **Deducción de Binarios desde `.desktop`:** Si el usuario elige un archivo `.desktop` incluido en el tarball, `tm` parsea el campo `Exec=` para encontrar el binario original y crear el symlink automáticamente.

### Integración con el Sistema (XDG)

  - **Control de Terminal:** Se añadió el campo `"terminal": bool` en los JSON de los repositorios. Esto permite definir si una app debe abrirse con o sin terminal (por defecto false).
  - **Parcheo de `.desktop`:** Toda aplicación instalada ahora fuerza `Terminal=false` (salvo que se indique lo contrario) para evitar que se abra una ventana de consola vacía al ejecutarla desde el menú de aplicaciones.
  - **Soporte pkexec:** Si un paquete requiere root (`requires_root: true`), el archivo `.desktop` antepone `pkexec` al comando de ejecución de forma automática.

### Ecosistema de Comunidad (Staging Area)

  - **Bandeja de Entrada (Invisible):** Se creó el directorio `assets/contributions/` en GitHub. Este espacio es exclusivo para que colaboradores envíen archivos JSON (uno por usuario) con múltiples paquetes.
  - **Aislamiento Total:** El código de Rust es agnóstico a esta carpeta. El mantenedor revisa los Pull Requests manualmente y "promociona" los paquetes aprobados a los repositorios oficiales.
  - **Consolidación Oficial:** Se establecieron `default_repos.json` y `community_repos.json` como las únicas fuentes oficiales sincronizadas por el binario.

### Nuevos Paquetes y Templates

  - **JetBrains Toolbox:** Se integró el soporte para este tipo de herramientas, asegurando que su URL y su ejecución gráfica funcionen correctamente bajo el nuevo sistema de metadatos.

## [1.5.0] - 2026-04-21

### Características (Features)
- **Perfil de Compilación Optimizada:** Se configuró un perfil `release` avanzado (`opt-level=3`, `lto=true`, `strip=true`) para generar binarios sustancialmente más rápidos y de menor tamaño.

### Mejoras de Rendimiento y Estructura (Performance & Refactoring)
- **Aislamiento I/O Asíncrono:** Funciones pesadas del núcleo como la extracción de tarballs y la configuración del sistema han sido migradas a `tokio::task::spawn_blocking`, solucionando de forma definitiva el "congelamiento" de la barra de progreso.
- **Caché Asíncrona en TUI:** El cálculo de tamaños de directorio y la vista previa de archivos grandes ahora se procesan en segundo plano, mostrando un estado temporal ("Loading preview...") y eliminando al 100% el *micro-stuttering* al navegar.
- **Limpieza de Reservas (Memory Allocation):** Refactorizado intensivo del bucle de renderizado para evitar clonación excesiva de cadenas de texto (`String::clone()`) en tiempo real, logrando una reducción masiva del consumo en memoria RAM.
- **Refactorización Global de Navegación:** Eliminado medio centenar de líneas de código duplicado e irrelevante en el gestor de ventanas (`handlers.rs`) mediante la creación de un interceptor de teclado limpio para popups genéricos.

## [1.4.4-1.4.6] - 2026-04-21

### Características (Features)
- **Migración a Arquitectura Asíncrona:** El núcleo del programa ha sido migrado completamente a un modelo asíncrono utilizando `Tokio` y `reqwest`. Esto mejora la estabilidad, la concurrencia y permite una TUI más fluida sin bloqueos durante operaciones pesadas de red.
- **Soporte Multi-Paquete en CLI:** Ahora es posible instalar o eliminar múltiples aplicaciones en un solo comando (ej: `tm install zen waterfox` o `tm remove discord vesktop`).
- **Vista Master-Detail (Repositorios):** Se rediseñó la interfaz de navegación de repositorios con un diseño de 40/60. Ahora puedes ver la lista a la izquierda y todos los detalles (incluyendo la nueva descripción) a la derecha.
- **Descripciones de Aplicaciones:** Se añadió un campo de descripción detallada para cada repositorio en los archivos `default_repos.json` y `community_repos.json`.
- **Visor de Logs Interno (F12):** Nueva herramienta de diagnóstico integrada que permite ver en tiempo real qué está ocurriendo "bajo el capó" (descargas, extracciones, errores detallados). Soporta scroll y navegación independiente.

### Mejoras (Improvements)
- **Comunicación entre Core y TUI:** Implementación de canales bidireccionales asíncronos (`tokio::sync::mpsc` y `oneshot`) para una gestión de eventos más limpia y profesional.
- **Ayuda Documentada:** Se agregó el acceso a los logs (`F12`) en el menú de ayuda global (`?`).
- **Prioridad de Metadatos:** En instalaciones masivas vía CLI, el sistema ahora prioriza automáticamente la configuración definida en los repositorios oficiales sobre los flags manuales, garantizando instalaciones correctas.

## [1.4.3] - 2026-04-20

### Características (Features)
- **Control Total sobre Binarios:** Se eliminó la autoselección automática de ejecutables. Ahora el usuario siempre debe confirmar qué binario desea vincular, incluso si solo se detecta uno en el tarball.
- **Opciones de Omisión (Skip):** Se agregó la capacidad de saltar la selección de binarios o de archivos `.desktop` durante la instalación interactiva, permitiendo una extracción "limpia" sin crear enlaces en el sistema.
- **Validación de Repositorios:** Ahora el sistema verifica que la URL sea válida y accesible antes de permitir agregar un nuevo repositorio personalizado, evitando errores futuros durante la instalación.
- **Nuevos Repositorios Oficiales:** Inclusión de **Zen Browser**, **Stoat**, **Discord** y **Floorp** en la lista de paquetes predeterminados.

### Mejoras (Improvements)
- **Robustez en la Selección:** Mejorado el manejo de canales de respuesta (`mpsc`) para evitar cierres inesperados de popups durante la instalación.

## [1.4.2] - 2026-04-20

### Características (Features)
- **Soporte para archivos `.desktop` oficiales:** Ahora `tm` detecta archivos `.desktop` dentro de los archivos comprimidos. El usuario puede elegir usar el oficial del desarrollador, el cual es parcheado dinámicamente para asegurar que los campos `Exec` e `Icon` apunten a las rutas correctas.
- **Detección Automática de Aplicaciones de Terminal:** Implementación de un escáner de dependencias dinámicas (`ldd`). Si un binario no tiene dependencias gráficas (GTK, Qt, X11, etc.), se marca automáticamente como `Terminal=true` en el acceso directo.
- **Confirmación de Actualización de `tm`:** El comando `--update-bin` ahora muestra una comparativa de la versión actual frente a la versión encontrada en GitHub y solicita confirmación antes de descargar el nuevo binario.

### Correcciones (Bug Fixes)
- **Sincronización de Repositorios:** Se corrigieron las URLs de GitHub en el comando `repo sync`, permitiendo que los archivos de configuración se descarguen correctamente desde la carpeta `assets/` del repositorio oficial.

## [1.4.1] - 2026-04-20

### Características y Mejoras de UI (Features & UI Improvements)
- **Instalaciones Asíncronas (Non-blocking):** Se eliminó por completo el congelamiento y suspensión de la terminal al instalar aplicaciones. Las instalaciones ahora ocurren en un hilo en segundo plano (background thread).
- **Barra de Progreso Nativa:** Integración de un widget `Gauge` interactivo en la TUI que muestra el porcentaje de descarga y el progreso de extracción en tiempo real para instalaciones tanto locales como desde repositorios.
- **Arquitectura Bidireccional de Canales (`mpsc`):** Nuevo sistema de mensajería (`InstallMessage`) que permite pausar la instalación para hacerle preguntas al usuario sin romper el renderizado de la terminal.
- **Selección Interactiva de Tarballs:** Cuando un repositorio de GitHub tiene múltiples archivos (ej. versiones ARM o DEB), la TUI ahora te permite seleccionar cuál descargar mediante un menú emergente fluido.
- **Selección de Binarios:** Si al extraer un archivo comprimido se detectan múltiples ejecutables, la interfaz abrirá un menú para que elijas cuál debe ser enlazado a tu sistema.
- **Interfaz más limpia:** El menú "Manage Repositories" se simplificó a "Repositories". Se mejoró el estilo del widget de progreso usando caracteres *Unicode* y un alto contraste fijo para mejor legibilidad.

### Correcciones (Bug Fixes)
- **Flickering y Desfase Visual Solucionado:** Las funciones internas de extracción y asignación fueron puestas en "Modo Silencioso" durante el uso de la TUI, evitando que impriman texto plano a `stdout` y rompan o desfasen el dibujo de los menús.
- **Navegación Corregida:** Arreglado el bug donde las teclas de flecha movían la lista de repositorios de fondo en lugar de interactuar con el popup de selección de tarballs.
- **Cursor de Repositorios:** Se corrigió un detalle visual donde la lista de categorías de repositorio no mostraba ninguna selección por defecto al entrar por primera vez.

## [1.4.0] - 2026-04-20

### Correcciones (Bug Fixes)
- **Descargas de Gran Tamaño:** Se migró el motor de descarga a un sistema de transmisión de flujo (streaming). Esto resuelve el error "error decoding response body" al descargar archivos pesados al evitar cargar todo el archivo en la memoria RAM.
- **Normalización de URLs:** Mejora en la extracción de nombres de archivos desde URLs que contienen parámetros de consulta o redirecciones complejas.

## [1.3.1-1.3.9] - 2026-04-19

### Mejoras e Integración (Improvements & Integration)
- **Perfeccionamiento del Sistema de Íconos:** Se rediseñó el buscador para soportar variantes de nombres con guiones (ej: `sublime-text.png` para "Sublime Text") y se aumentó la prioridad de las rutas estándar como `/icons/`, `/share/icons/` y `/pixmaps/`.
- **Soporte para Aplicaciones Electron:** Se eliminaron las restricciones de búsqueda en carpetas `/build/` y `/out/`, permitiendo que aplicaciones como *Heroic Launcher* detecten sus íconos correctamente.
- **Normalización de Accesos Directos:** Los archivos `.desktop` ahora usan el nombre del repositorio (ej: "Reaper") en lugar del nombre del paquete técnico para el campo visual del menú.
- **Sanitización de Archivos:** Tanto los archivos `.desktop` como los binarios en `~/.local/bin` ahora usan nombres normalizados (minúsculas y sin espacios) para garantizar la compatibilidad con todos los lanzadores y terminales.
- **Refresco Automático:** Integración de `update-desktop-database` tras cada instalación para que los cambios en íconos y menús se reflejen instantáneamente sin necesidad de reiniciar la sesión.
- **Compatibilidad Ampliada:** Soporte restaurado para íconos `.ico` y nuevo soporte para archivos `.xpm`.

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
