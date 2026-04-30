//! NixOS integration for KPM.
//!
//! When running on NixOS, this module generates a Nix flake (package.nix + flake.nix)
//! for each installed application and installs it via `nix profile add`.
//! This ensures all library dependencies are resolved correctly through the Nix store.

use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Runtime type
// ---------------------------------------------------------------------------

/// Describes the primary runtime environment an installed app needs.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeType {
    /// Native ELF binary — uses `autoPatchelfHook`.
    Elf,
    /// Pure Python scripts — uses `makeWrapper + python3`.
    Python,
    /// Pure Lua scripts — uses `makeWrapper + lua5_4`.
    Lua,
    /// Shell scripts with no specific interpreter.
    Shell,
    /// ELF + Python scripts — uses both hooks.
    MixedPython,
    /// ELF + Lua scripts.
    MixedLua,
}

/// GPU / graphics-stack mode detected for a package.
#[derive(Debug, Clone, PartialEq)]
pub enum GpuMode {
    /// No special GPU handling needed.
    None,
    /// WebKit/Tauri app: needs LD_LIBRARY_PATH with webkit + compositing disabled.
    WebKit,
    /// Electron/Chromium app that bundles its own EGL driver:
    /// needs /run/opengl-driver bind + LD_LIBRARY_PATH with mesa.
    Electron,
}

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

/// Returns `true` when the current system is NixOS.
pub fn is_nixos() -> bool {
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        content.lines().any(|l| {
            let l = l.trim();
            l == "ID=nixos" || l.starts_with("ID=\"nixos\"")
        })
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// GPU mode detection
// ---------------------------------------------------------------------------

/// Scan an installed directory to detect if the app needs special GPU handling.
///
/// - Looks for WebKit dlopen strings in ELF files → `GpuMode::WebKit`
/// - Looks for bundled `libEGL.so` / `chrome-sandbox` / Electron markers → `GpuMode::Electron`
pub fn detect_gpu_mode(dir: &Path) -> GpuMode {
    let mut has_webkit   = false;
    let mut has_electron = false;

    // Check for obvious Electron markers first (fast path)
    if dir.join("chrome-sandbox").exists()
        || dir.join("electron").exists()
        || dir.join("resources").join("app.asar").exists()
        // Bundled libEGL is the clearest sign an Electron app ships its own GPU stack
        || dir.join("libEGL.so").exists()
        || dir.join("libEGL.so.1").exists()
    {
        has_electron = true;
    }

    // Deep scan ELF files for dlopen strings
    for entry in walkdir::WalkDir::new(dir).max_depth(5).follow_links(false) {
        let entry = match entry { Ok(e) => e, Err(_) => continue };
        if !entry.file_type().is_file() { continue; }
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();

        // A bundled libEGL with version suffix also counts
        if name.starts_with("libEGL") && (name.ends_with(".so") || name.contains(".so.")) {
            has_electron = true;
        }

        if let Ok(bytes) = fs::read(path) {
            if bytes.len() < 4 || &bytes[..4] != b"\x7fELF" { continue; }
            // WebKit dlopen strings
            if bytes_contain(&bytes, b"libwebkit2gtk-4.1")
                || bytes_contain(&bytes, b"libwebkit2gtk-4.0")
                || bytes_contain(&bytes, b"libjavascriptcoregtk")
            {
                has_webkit = true;
            }
            // Electron/Chromium strings
            if bytes_contain(&bytes, b"ELECTRON_RUN_AS_NODE")
                || bytes_contain(&bytes, b"app.asar")
                || bytes_contain(&bytes, b"chrome-sandbox")
            {
                has_electron = true;
            }
        }
    }

    if has_webkit   { return GpuMode::WebKit;   }
    if has_electron { return GpuMode::Electron; }
    GpuMode::None
}

// ---------------------------------------------------------------------------
// ELF scanning
// ---------------------------------------------------------------------------

/// Recursively scan a directory for ELF files and collect their NEEDED sonames.
pub fn scan_needed_libs(root: &Path) -> Result<BTreeSet<String>> {
    let mut libs = BTreeSet::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let p = entry.path();
        if let Ok(needed) = needed_libs_for_file(p) {
            for n in needed {
                libs.insert(n);
            }
        }
    }
    Ok(libs)
}

/// Read the NEEDED entries from a single ELF file.
fn needed_libs_for_file(path: &Path) -> Result<Vec<String>> {
    let bytes = fs::read(path)?;
    if bytes.len() < 4 || &bytes[..4] != b"\x7fELF" {
        return Ok(vec![]);
    }
    let elf = goblin::elf::Elf::parse(&bytes)?;
    let needed: Vec<String> = elf
        .libraries
        .iter()
        .map(|s| s.to_string())
        .collect();
    Ok(needed)
}

// ---------------------------------------------------------------------------
// Soname → nixpkgs mapping
// ---------------------------------------------------------------------------

/// Map a shared-library soname to its nixpkgs attribute (if known).
fn soname_to_nixpkg(soname: &str) -> Option<&'static str> {
    let base = soname.split(".so").next().unwrap_or(soname);
    // Prefix-based rules first (for versioned names like libpython3.11)
    if base.starts_with("libpython3")  { return Some("python3"); }
    if base.starts_with("libpython2")  { return Some("python2"); }
    if base.starts_with("libluajit")   { return Some("luajit"); }
    if base.starts_with("liblua")      { return Some("lua5_4"); }
    if base.starts_with("libruby")     { return Some("ruby"); }
    if base.starts_with("libnode")     { return Some("nodejs"); }
    if base.starts_with("libperl")     { return Some("perl"); }
    match base {
        // Core system
        "libc" | "libm" | "libdl" | "librt" | "libpthread" | "libutil" | "libresolv" => Some("glibc"),
        "libstdc++" => Some("stdenv.cc.cc.lib"),
        "libgcc_s"  => Some("gcc-unwrapped.lib"),
        "libz"      => Some("zlib"),
        "libbz2"    => Some("bzip2"),
        "liblzma"   => Some("xz"),
        "libffi"    => Some("libffi"),
        "libexpat"  => Some("expat"),
        "libpcre2-8" | "libpcre" => Some("pcre2"),
        "libsqlite3" => Some("sqlite"),
        // X11 / Display
        "libX11" | "libX11-xcb" => Some("xorg.libX11"),
        "libXext"        => Some("xorg.libXext"),
        "libXrandr"      => Some("xorg.libXrandr"),
        "libXi"          => Some("xorg.libXi"),
        "libXcursor"     => Some("xorg.libXcursor"),
        "libXfixes"      => Some("xorg.libXfixes"),
        "libXrender"     => Some("xorg.libXrender"),
        "libXcomposite"  => Some("xorg.libXcomposite"),
        "libXdamage"     => Some("xorg.libXdamage"),
        "libXtst"        => Some("xorg.libXtst"),
        "libXinerama"    => Some("xorg.libXinerama"),
        "libXScrnSaver"  => Some("xorg.libXScrnSaver"),
        "libXau"         => Some("xorg.libXau"),
        "libxcb"         => Some("xorg.libxcb"),
        "libxkbcommon"   => Some("libxkbcommon"),
        // Wayland
        "libwayland-client" | "libwayland-server"
        | "libwayland-cursor" | "libwayland-egl" => Some("wayland"),
        // GL / Vulkan
        "libGL" | "libGLX" | "libEGL" | "libOpenGL" | "libGLESv2" => Some("libGL"),
        "libvulkan"     => Some("vulkan-loader"),
        "libGLdispatch" => Some("libGL"),
        "libdrm"        => Some("libdrm"),
        "libgbm"        => Some("mesa"),
        "libepoxy"      => Some("libepoxy"),
        // GTK / GLib / GDK
        "libgtk-3" | "libgdk-3"  => Some("gtk3"),
        "libgtk-4"               => Some("gtk4"),
        "libglib-2.0" | "libgio-2.0" | "libgmodule-2.0"
        | "libgobject-2.0" | "libgthread-2.0" => Some("glib"),
        "libgdk_pixbuf-2.0"      => Some("gdk-pixbuf"),
        "libpango-1.0" | "libpangocairo-1.0" | "libpangoft2-1.0" => Some("pango"),
        "libcairo" | "libcairo-gobject" => Some("cairo"),
        "libatk-1.0"   => Some("atk"),
        "libharfbuzz"  => Some("harfbuzz"),
        "libfontconfig" => Some("fontconfig"),
        "libfreetype"  => Some("freetype"),
        // WebKit
        "libwebkit2gtk-4.0" | "libwebkit2gtk-4.1"
        | "libjavascriptcoregtk-4.0" | "libjavascriptcoregtk-4.1" => Some("webkitgtk_4_1"),
        "libsoup-3.0" => Some("libsoup_3"),
        "libsoup-2.4" => Some("libsoup"),
        // Qt
        "libQt5Core" | "libQt5Gui" | "libQt5Widgets"
        | "libQt5Network" | "libQt5DBus" => Some("qt5.qtbase"),
        "libQt5Svg"    => Some("qt5.qtsvg"),
        "libQt5WebEngine" | "libQt5WebEngineCore"
        | "libQt5WebEngineWidgets" => Some("qt5.qtwebengine"),
        "libQt6Core" | "libQt6Gui" | "libQt6Widgets"
        | "libQt6Network" | "libQt6DBus" => Some("qt6.qtbase"),
        // Audio
        "libasound"              => Some("alsa-lib"),
        "libpulse" | "libpulse-simple" => Some("libpulseaudio"),
        "libsndfile"             => Some("libsndfile"),
        "libpipewire-0.3"        => Some("pipewire"),
        // Crypto / SSL
        "libssl" | "libcrypto" => Some("openssl"),
        "libgnutls"            => Some("gnutls"),
        // Media
        "libavcodec" | "libavformat" | "libavutil"
        | "libswresample" | "libswscale" => Some("ffmpeg"),
        "libpng16" | "libpng" => Some("libpng"),
        "libjpeg"             => Some("libjpeg"),
        "libwebp"             => Some("libwebp"),
        "libtiff"             => Some("libtiff"),
        // Network / IPC
        "libcurl"   => Some("curl"),
        "libdbus-1" => Some("dbus"),
        "libnss3" | "libnssutil3" | "libsmime3"
        | "libnspr4" | "libplc4" | "libplds4" => Some("nss"),
        // Misc
        "libudev"          => Some("systemd"),
        "libnotify"        => Some("libnotify"),
        "libsecret-1"      => Some("libsecret"),
        "libappindicator3" => Some("libappindicator-gtk3"),
        "libatspi"         => Some("at-spi2-core"),
        "libfuse"          => Some("fuse"),
        "libinput"         => Some("libinput"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Nix file generation
// ---------------------------------------------------------------------------

/// Generate a `package.nix` for a tarball-extracted directory.
///
/// Uses `autoPatchelfHook` to automatically resolve library dependencies.
fn render_tarball_package_nix(
    pname: &str,
    installed_dir: &Path,
    bin_names: &[String],
    deps: &[String],
    gpu_mode: &GpuMode,
) -> String {
    // Build the inputs list for callPackage
    let mut inputs = vec![
        "lib".to_string(),
        "stdenv".to_string(),
        "autoPatchelfHook".to_string(),
    ];
    for d in deps {
        // Extract the last component (e.g. "xorg.libX11" → add "xorg" at top level)
        let top = d.split('.').next().unwrap_or(d);
        if !inputs.contains(&top.to_string()) {
            inputs.push(top.to_string());
        }
    }

    // For GPU-accelerated apps we need makeWrapper; ensure it's in inputs
    let needs_wrapper = *gpu_mode != GpuMode::None;
    if needs_wrapper && !inputs.contains(&"makeWrapper".to_string()) {
        inputs.push("makeWrapper".to_string());
    }
    if *gpu_mode == GpuMode::Electron || *gpu_mode == GpuMode::WebKit {
        for pkg in &["libGL", "mesa", "libdrm"] {
            let s = pkg.to_string();
            if !inputs.contains(&s) { inputs.push(s.clone()); }
        }
    }
    if *gpu_mode == GpuMode::WebKit {
        for pkg in &["webkitgtk_4_1", "libsoup_3"] {
            let s = pkg.to_string();
            if !inputs.contains(&s) { inputs.push(s.clone()); }
        }
    }

    let inputs_str   = inputs.iter().map(|s| format!("  {}", s)).collect::<Vec<_>>().join(",\n");
    let build_inputs = deps.join("\n    ");

    // Generate bin/ entries: plain symlinks for generic, makeWrapper for GPU apps
    let bin_links: String = match gpu_mode {
        GpuMode::WebKit => bin_names.iter().map(|b| {
            let base = std::path::Path::new(b).file_name().unwrap_or_default().to_string_lossy();
            format!("    makeWrapper $out/{b} $out/bin/{base} \\\
\n      --set-default LD_LIBRARY_PATH (lib.makeLibraryPath [ webkitgtk_4_1 libsoup_3 mesa libGL libdrm ]) \\\
\n      --set-default WEBKIT_DISABLE_COMPOSITING_MODE 1 \\\
\n      --set-default WEBKIT_DISABLE_DMABUF_RENDERER 1")
        }).collect::<Vec<_>>().join("\n"),
        GpuMode::Electron => bin_names.iter().map(|b| {
            let base = std::path::Path::new(b).file_name().unwrap_or_default().to_string_lossy();
            format!("    makeWrapper $out/{b} $out/bin/{base} \\\
\n      --set-default LD_LIBRARY_PATH \"/run/opengl-driver/lib:$(makeLibraryPath [ libGL mesa libdrm ])\" \\\
\n      --set-default LIBGL_DRIVERS_PATH \"/run/opengl-driver/lib/dri\" \\\
\n      --set-default __EGL_VENDOR_LIBRARY_DIRS \"/run/opengl-driver/share/glvnd/egl_vendor.d\"")
        }).collect::<Vec<_>>().join("\n"),
        GpuMode::None => bin_names.iter()
            .map(|b| format!("    ln -sf $out/{b} $out/bin/{b}"))
            .collect::<Vec<_>>().join("\n"),
    };

    let native_build = if needs_wrapper {
        "autoPatchelfHook makeWrapper"
    } else {
        "autoPatchelfHook"
    };

    format!(
        r#"{{
{inputs_str}
}}:

stdenv.mkDerivation {{
  pname = "{pname}";
  version = "0.0.0";

  src = builtins.path {{
    path = {src_path};
    name = "{pname}-src";
  }};

  nativeBuildInputs = [ {native_build} ];

  buildInputs = [
    {build_inputs}
  ];

  dontConfigure = true;
  dontBuild = true;

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin
    cp -a . $out/
{bin_links}
    runHook postInstall
  '';

  # Remove dangling symlinks that would fail noBrokenSymlinks check
  postFixup = ''
    find $out -xtype l -delete 2>/dev/null || true
  '';

  meta = with lib; {{
    description = "{pname} installed via KPM";
    platforms = [ "x86_64-linux" "aarch64-linux" ];
    mainProgram = "{main_bin}";
  }};
}}
"#,
        inputs_str   = inputs_str,
        pname        = pname,
        src_path     = installed_dir.display(),
        build_inputs = build_inputs,
        bin_links    = bin_links,
        native_build = native_build,
        main_bin     = bin_names.first().map(|s| s.as_str()).unwrap_or(pname),
    )
}

/// Generate a `package.nix` for a Python/script-based app.
///
/// Uses `makeWrapper` to wrap script executables with the right Python in PATH.
fn render_python_package_nix(
    pname: &str,
    installed_dir: &Path,
    bin_names: &[String],
) -> String {
    let bin_links = bin_names
        .iter()
        .map(|b| format!("    makeWrapper $out/{b} $out/bin/{main} --prefix PATH : ${{python3}}/bin",
            b = b,
            main = std::path::Path::new(b).file_name().unwrap_or_default().to_string_lossy()))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"{{
  lib,
  stdenv,
  python3,
  makeWrapper,
}}:

stdenv.mkDerivation {{
  pname = "{pname}";
  version = "0.0.0";

  src = builtins.path {{
    path = {src_path};
    name = "{pname}-src";
  }};

  nativeBuildInputs = [ makeWrapper ];
  buildInputs = [ python3 ];

  dontConfigure = true;
  dontBuild = true;

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin
    cp -a . $out/
{bin_links}
    runHook postInstall
  '';

  # Remove dangling symlinks that would fail noBrokenSymlinks check
  postFixup = ''
    find $out -xtype l -delete 2>/dev/null || true
  '';

  meta = with lib; {{
    description = "{pname} installed via KPM";
    platforms = [ "x86_64-linux" "aarch64-linux" ];
    mainProgram = "{main_bin}";
  }};
}}
"#,
        pname = pname,
        src_path = installed_dir.display(),
        bin_links = bin_links,
        main_bin = bin_names.first()
            .map(|s| std::path::Path::new(s).file_name().unwrap_or_default().to_string_lossy().to_string())
            .unwrap_or_else(|| pname.to_string()),
    )
}

/// Generate a `package.nix` for a Lua-based app.
fn render_lua_package_nix(pname: &str, installed_dir: &Path, bin_names: &[String]) -> String {
    let bin_links = bin_names.iter().map(|b| {
        let base = std::path::Path::new(b).file_name().unwrap_or_default().to_string_lossy();
        format!("    makeWrapper $out/{b} $out/bin/{base} --prefix PATH : ${{lua5_4}}/bin")
    }).collect::<Vec<_>>().join("\n");
    format!(
        r#"{{
  lib,
  stdenv,
  lua5_4,
  makeWrapper,
}}:

stdenv.mkDerivation {{
  pname = "{pname}";
  version = "0.0.0";

  src = builtins.path {{
    path = {src};
    name = "{pname}-src";
  }};

  nativeBuildInputs = [ makeWrapper ];
  buildInputs = [ lua5_4 ];

  dontConfigure = true;
  dontBuild = true;

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin
    cp -a . $out/
{bin_links}
    runHook postInstall
  '';

  # Remove dangling symlinks that would fail noBrokenSymlinks check
  postFixup = ''
    find $out -xtype l -delete 2>/dev/null || true
  '';

  meta = with lib; {{
    description = "{pname} installed via KPM";
    platforms = [ "x86_64-linux" "aarch64-linux" ];
    mainProgram = "{main}";
  }};
}}
"#,
        pname = pname, src = installed_dir.display(), bin_links = bin_links,
        main = bin_names.first()
            .map(|s| std::path::Path::new(s).file_name().unwrap_or_default().to_string_lossy().to_string())
            .unwrap_or_else(|| pname.to_string()),
    )
}

/// Generate a `package.nix` for a mixed ELF + interpreter app (e.g. ranger).
/// Uses `autoPatchelfHook` for native libs and `makeWrapper` for the interpreter.
fn render_mixed_package_nix(
    pname: &str,
    installed_dir: &Path,
    bin_names: &[String],
    interp_pkg: &str,  // e.g. "python3" or "lua5_4"
    deps: &[String],
    gpu_mode: &GpuMode,
) -> String {
    let mut inputs = vec![
        "lib".to_string(), "stdenv".to_string(),
        "autoPatchelfHook".to_string(), "makeWrapper".to_string(),
        interp_pkg.to_string(),
    ];
    for d in deps {
        let top = d.split('.').next().unwrap_or(d);
        if !inputs.contains(&top.to_string()) { inputs.push(top.to_string()); }
    }
    // Add GPU packages when needed
    if *gpu_mode == GpuMode::Electron || *gpu_mode == GpuMode::WebKit {
        for pkg in &["libGL", "mesa", "libdrm"] {
            let s = pkg.to_string();
            if !inputs.contains(&s) { inputs.push(s); }
        }
    }
    if *gpu_mode == GpuMode::WebKit {
        for pkg in &["webkitgtk_4_1", "libsoup_3"] {
            let s = pkg.to_string();
            if !inputs.contains(&s) { inputs.push(s); }
        }
    }

    let inputs_str = inputs.iter().map(|s| format!("  {}", s)).collect::<Vec<_>>().join(",\n");
    let all_deps = std::iter::once(interp_pkg.to_string())
        .chain(deps.iter().cloned()).collect::<Vec<_>>().join("\n    ");

    // Extend makeWrapper flags with GPU env vars when needed
    let gpu_flags = match gpu_mode {
        GpuMode::WebKit => format!(
            " --set-default LD_LIBRARY_PATH \"${{webkitgtk_4_1}}/lib:${{libsoup_3}}/lib:${{mesa}}/lib:${{libGL}}/lib:${{libdrm}}/lib\" \\
      --set-default WEBKIT_DISABLE_COMPOSITING_MODE 1 \\
      --set-default WEBKIT_DISABLE_DMABUF_RENDERER 1"
        ),
        GpuMode::Electron => format!(
            " --set-default LD_LIBRARY_PATH \"/run/opengl-driver/lib:${{mesa}}/lib:${{libGL}}/lib:${{libdrm}}/lib\" \\
      --set-default LIBGL_DRIVERS_PATH \"/run/opengl-driver/lib/dri\" \\
      --set-default __EGL_VENDOR_LIBRARY_DIRS \"/run/opengl-driver/share/glvnd/egl_vendor.d\""
        ),
        GpuMode::None => String::new(),
    };

    let bin_links = bin_names.iter().map(|b| {
        let base = std::path::Path::new(b).file_name().unwrap_or_default().to_string_lossy();
        format!("    makeWrapper $out/{b} $out/bin/{base} --prefix PATH : ${{{interp_pkg}}}/bin{gpu_flags}",
            gpu_flags = gpu_flags)
    }).collect::<Vec<_>>().join("\n");
    format!(
        r#"{{
{inputs_str},
}}:

stdenv.mkDerivation {{
  pname = "{pname}";
  version = "0.0.0";

  src = builtins.path {{
    path = {src};
    name = "{pname}-src";
  }};

  nativeBuildInputs = [ autoPatchelfHook makeWrapper ];
  buildInputs = [
    {all_deps}
  ];

  dontConfigure = true;
  dontBuild = true;

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin
    cp -a . $out/
{bin_links}
    runHook postInstall
  '';

  # Remove dangling symlinks that would fail noBrokenSymlinks check
  postFixup = ''
    find $out -xtype l -delete 2>/dev/null || true
  '';

  meta = with lib; {{
    description = "{pname} installed via KPM";
    platforms = [ "x86_64-linux" "aarch64-linux" ];
    mainProgram = "{main}";
  }};
}}
"#,
        inputs_str = inputs_str, pname = pname, src = installed_dir.display(),
        all_deps = all_deps, bin_links = bin_links,
        main = bin_names.first()
            .map(|s| std::path::Path::new(s).file_name().unwrap_or_default().to_string_lossy().to_string())
            .unwrap_or_else(|| pname.to_string()),
    )
}

// ---------------------------------------------------------------------------
// AppImage extra-dependency detection (dlopen / Electron / WebKit)
// ---------------------------------------------------------------------------

/// Returns `true` if `haystack` contains the byte sequence `needle`.
fn bytes_contain(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Scan extracted AppImage squashfs for libraries loaded via `dlopen` (absent
/// from ELF NEEDED) and for well-known Electron/WebKit app indicators.
/// Returns extra nixpkgs attrs to merge into `extraPkgs`.
fn detect_appimage_extras(squashfs_root: &Path) -> Vec<&'static str> {
    let mut extras: Vec<&'static str> = Vec::new();
    let mut add = |pkg: &'static str| {
        if !extras.contains(&pkg) { extras.push(pkg); }
    };
    // Electron / Chromium indicators
    if squashfs_root.join("chrome-sandbox").exists()
        || squashfs_root.join("electron").exists()
        || squashfs_root.join("resources").join("app.asar").exists()
    {
        add("nss"); add("at-spi2-core"); add("mesa"); add("libdrm");
    }
    // ELF byte-string search for dlopen targets
    for entry in walkdir::WalkDir::new(squashfs_root).max_depth(5).follow_links(false) {
        let entry = match entry { Ok(e) => e, Err(_) => continue };
        if !entry.file_type().is_file() { continue; }
        if let Ok(bytes) = fs::read(entry.path()) {
            if bytes.len() < 4 || &bytes[..4] != b"\x7fELF" { continue; }
            if bytes_contain(&bytes, b"libwebkit2gtk-4.1")
                || bytes_contain(&bytes, b"libwebkit2gtk-4.0")
                || bytes_contain(&bytes, b"libjavascriptcoregtk")
            {
                add("webkitgtk_4_1");
            }
            if bytes_contain(&bytes, b"libsoup-3")   { add("libsoup_3"); }
            if bytes_contain(&bytes, b"libvulkan")   { add("vulkan-loader"); }
        }
    }
    // Packed/self-decompressing binary heuristic:
    // If no .so files exist in squashfs the binary may be UPX-packed or similar
    // and rely entirely on system libs. Infer deps from the .desktop categories.
    let has_bundled_libs = walkdir::WalkDir::new(squashfs_root)
        .max_depth(5)
        .into_iter()
        .flatten()
        .any(|e| {
            let n = e.file_name().to_string_lossy();
            e.file_type().is_file() && (n.ends_with(".so") || n.contains(".so."))
        });
    if !has_bundled_libs {
        let desktop_text: String = walkdir::WalkDir::new(squashfs_root)
            .max_depth(3)
            .into_iter()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".desktop"))
            .filter_map(|e| fs::read_to_string(e.path()).ok())
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        // GUI / web-view app with packed binary → very likely needs WebKit
        if desktop_text.contains("audio")
            || desktop_text.contains("video")
            || desktop_text.contains("network")
            || desktop_text.contains("internet")
            || desktop_text.contains("web")
        {
            add("webkitgtk_4_1");
            add("libsoup_3");
        }
    }
    extras
}

/// Scan the raw AppImage bytes for known library soname strings.
/// Used as a fallback when FUSE extraction is unavailable, and as extra
/// insurance alongside extraction-based scanning.
fn scan_appimage_raw(appimage_path: &Path) -> Vec<&'static str> {
    let mut extras: Vec<&'static str> = Vec::new();
    let mut add = |pkg: &'static str| {
        if !extras.contains(&pkg) { extras.push(pkg); }
    };
    let Ok(bytes) = fs::read(appimage_path) else { return extras; };
    // WebKit / Tauri
    if bytes_contain(&bytes, b"libwebkit2gtk-4.1")
        || bytes_contain(&bytes, b"libwebkit2gtk-4.0")
        || bytes_contain(&bytes, b"libjavascriptcoregtk")
    {
        add("webkitgtk_4_1");
    }
    if bytes_contain(&bytes, b"libsoup-3")    { add("libsoup_3"); }
    if bytes_contain(&bytes, b"libvulkan")    { add("vulkan-loader"); }
    // Electron / Chromium
    if bytes_contain(&bytes, b"chrome-sandbox")
        || bytes_contain(&bytes, b"app.asar")
        || bytes_contain(&bytes, b"ELECTRON_RUN_AS_NODE")
    {
        add("nss"); add("at-spi2-core"); add("mesa"); add("libdrm");
    }
    // Qt WebEngine
    if bytes_contain(&bytes, b"QtWebEngine") || bytes_contain(&bytes, b"libQt5WebEngine") {
        add("qt5.qtwebengine");
    }
    extras
}

/// Generate a `package.nix` for an AppImage using `appimageTools.wrapType2`.
/// `deps` are the nixpkgs attrs detected by scanning the extracted AppImage.
/// `extras` are additional pkgs found via dlopen/Electron scanning.
fn render_appimage_package_nix(pname: &str, deps: &[String], extras: &[&str]) -> String {
    let mut all_pkgs: Vec<String> = deps.to_vec();
    for e in extras {
        let s = e.to_string();
        if !all_pkgs.contains(&s) { all_pkgs.push(s); }
    }

    // Detect app type for specialised sandbox environment
    let has_webkit   = all_pkgs.iter().any(|p| p.contains("webkitgtk"));
    let has_electron = all_pkgs.iter().any(|p| p == "nss" || p == "mesa" || p == "libdrm");

    // Electron apps always need mesa + libGL for EGL
    if has_electron {
        for pkg in &["libGL", "mesa", "libdrm"] {
            let s = pkg.to_string();
            if !all_pkgs.contains(&s) { all_pkgs.push(s); }
        }
    }
    // WebKit apps always need libsoup_3
    if has_webkit {
        if !all_pkgs.contains(&"libsoup_3".to_string()) { all_pkgs.push("libsoup_3".to_string()); }
    }

    // Build the inputs list from all packages
    let mut inputs = vec![
        "lib".to_string(), "appimageTools".to_string(),
        "glib-networking".to_string(), "gst_all_1".to_string(),
    ];
    for d in &all_pkgs {
        let top = d.split('.').next().unwrap_or(d);
        if !inputs.contains(&top.to_string()) { inputs.push(top.to_string()); }
    }

    let inputs_str   = inputs.iter().map(|s| format!("  {}", s)).collect::<Vec<_>>().join(",\n");
    let runtime_libs = all_pkgs.join("\n    ");

    // Specialised extraBwrapArgs per app type
    let extra_bwrap = if has_webkit {
        // Tauri / WebKit: expose all libs via LD_LIBRARY_PATH, disable
        // compositing and dmabuf which crash inside the bwrap sandbox.
        r#"    "--setenv" "GIO_EXTRA_MODULES" "${{glib-networking}}/lib/gio/modules"
    "--setenv" "GST_PLUGIN_PATH" (lib.makeSearchPathOutput "lib" "lib/gstreamer-1.0" gstPlugins)
    "--setenv" "LD_LIBRARY_PATH" (lib.makeLibraryPath (runtimeLibs ++ gstPlugins))
    "--setenv" "WEBKIT_DISABLE_COMPOSITING_MODE" "1"
    "--setenv" "WEBKIT_DISABLE_DMABUF_RENDERER" "1""#.to_string()
    } else if has_electron {
        // Electron / Chromium: bind the NixOS GPU driver path so libEGL.so.1
        // and the Mesa DRI drivers are visible inside bwrap.
        r#"    "--setenv" "GIO_EXTRA_MODULES" "${{glib-networking}}/lib/gio/modules"
    "--setenv" "GST_PLUGIN_PATH" (lib.makeSearchPathOutput "lib" "lib/gstreamer-1.0" gstPlugins)
    "--setenv" "LD_LIBRARY_PATH" (lib.makeLibraryPath (runtimeLibs ++ gstPlugins))
    "--ro-bind-try" "/run/opengl-driver" "/run/opengl-driver"
    "--setenv" "LIBGL_DRIVERS_PATH" "/run/opengl-driver/lib/dri"
    "--setenv" "__EGL_VENDOR_LIBRARY_DIRS" "/run/opengl-driver/share/glvnd/egl_vendor.d""#.to_string()
    } else {
        // Generic AppImage
        r#"    "--setenv" "GIO_EXTRA_MODULES" "${{glib-networking}}/lib/gio/modules"
    "--setenv" "GST_PLUGIN_PATH" (lib.makeSearchPathOutput "lib" "lib/gstreamer-1.0" gstPlugins)"#.to_string()
    };

    format!(
        r#"{{
{inputs_str},
}}:

let
  pname = "{pname}";
  version = "0.0.0";
  src = ./source.AppImage;

  runtimeLibs = [
    {runtime_libs}
  ];

  gstPlugins = with gst_all_1; [
    gstreamer
    gst-plugins-base
    gst-plugins-good
  ];
in
appimageTools.wrapType2 {{
  inherit pname version src;

  extraPkgs = pkgs: runtimeLibs ++ gstPlugins;

  extraBwrapArgs = [
{extra_bwrap}
  ];

  meta = {{
    description = "{pname} installed via KPM";
    mainProgram = "{pname}";
    platforms = [ "x86_64-linux" ];
  }};
}}
"#,
        inputs_str   = inputs_str,
        pname        = pname,
        runtime_libs = runtime_libs,
        extra_bwrap  = extra_bwrap,
    )
}

/// Generate a standard `flake.nix` that wraps `package.nix`.
fn render_flake_nix() -> String {
    r#"{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }: let
    systems = [ "x86_64-linux" "aarch64-linux" ];
    forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f (import nixpkgs {
      inherit system;
      config.allowUnfree = true;
    }));
  in {
    packages = forAllSystems (pkgs: {
      default = pkgs.callPackage ./package.nix {};
    });
  };
}
"#
    .to_string()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn sanitize_attr_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

/// Find relative paths of executable files in a directory (for bin/ links).
fn pick_bin_names(installed_dir: &Path) -> Vec<String> {
    let mut bins = Vec::new();
    for entry in walkdir::WalkDir::new(installed_dir)
        .max_depth(3)
        .follow_links(false)
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let p = entry.path();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = p.metadata() {
                if meta.permissions().mode() & 0o111 == 0 {
                    continue;
                }
            }
        }
        if let Ok(bytes) = fs::read(p) {
            let is_elf = bytes.len() >= 4 && &bytes[..4] == b"\x7fELF";
            let is_script = bytes.len() >= 2 && &bytes[..2] == b"#!";
            if is_elf || is_script {
                if let Ok(rel) = p.strip_prefix(installed_dir) {
                    bins.push(rel.to_string_lossy().to_string());
                }
            }
        }
    }
    bins
}

/// Detect the primary runtime type of an installed directory.
fn detect_runtime_type(installed_dir: &Path) -> RuntimeType {
    let mut python_count = 0u32;
    let mut lua_count    = 0u32;
    let mut elf_count    = 0u32;
    for entry in walkdir::WalkDir::new(installed_dir).max_depth(3).follow_links(false) {
        let entry = match entry { Ok(e) => e, Err(_) => continue };
        if !entry.file_type().is_file() { continue; }
        let p = entry.path();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(m) = p.metadata() {
                if m.permissions().mode() & 0o111 == 0 { continue; }
            }
        }
        if let Ok(bytes) = fs::read(p) {
            if bytes.len() >= 4 && &bytes[..4] == b"\x7fELF" {
                elf_count += 1;
            } else if bytes.len() >= 2 && bytes[0] == b'#' && bytes[1] == b'!' {
                let header = String::from_utf8_lossy(&bytes[..bytes.len().min(120)]);
                let shebang = header.lines().next().unwrap_or("");
                if shebang.contains("python") {
                    python_count += 1;
                } else if shebang.contains("lua") {
                    lua_count += 1;
                }
            }
        }
    }
    match (elf_count, python_count, lua_count) {
        (0, p, _) if p > 0 => RuntimeType::Python,
        (0, _, l) if l > 0 => RuntimeType::Lua,
        (0, 0, 0)          => RuntimeType::Shell,
        (_, p, _) if p > 0 => RuntimeType::MixedPython,
        (_, _, l) if l > 0 => RuntimeType::MixedLua,
        _                  => RuntimeType::Elf,
    }
}

// ---------------------------------------------------------------------------
// Public API: nixify + install
// ---------------------------------------------------------------------------

/// Nixify an already-extracted tarball folder.
///
/// 1. Scans for ELF dependencies
/// 2. Generates `package.nix` with `autoPatchelfHook` + detected deps
/// 3. Generates `flake.nix`
/// 4. Runs `nix profile add`
///
/// Returns `Ok(true)` on success, `Ok(false)` on nix build failure.
pub fn nixify_installed_folder(
    installed_dir: &Path,
    pname: &str,
    log_fn: Option<&dyn Fn(&str)>,
) -> Result<bool> {
    let log = |msg: &str| {
        if let Some(f) = log_fn {
            f(msg);
        }
    };

    let attr = sanitize_attr_name(pname);

    let out_dir = directories::BaseDirs::new()
        .context("Could not determine home directory")?
        .home_dir()
        .join(".kpm-nix")
        .join(&attr);
    fs::create_dir_all(&out_dir)?;

    let runtime = detect_runtime_type(installed_dir);
    log(&format!("Detected runtime type: {:?}", runtime));

    let bins = pick_bin_names(installed_dir);
    if bins.is_empty() {
        log("  Warning: no executable binaries found");
    } else {
        log(&format!("  Found {} executable(s)", bins.len()));
    }

    let package_nix = match &runtime {
        RuntimeType::Python => {
            log("Generating Nix derivation (makeWrapper + python3)...");
            render_python_package_nix(pname, installed_dir, &bins)
        }
        RuntimeType::Lua => {
            log("Generating Nix derivation (makeWrapper + lua5_4)...");
            render_lua_package_nix(pname, installed_dir, &bins)
        }
        RuntimeType::MixedPython | RuntimeType::MixedLua => {
            let interp_pkg = if runtime == RuntimeType::MixedLua { "lua5_4" } else { "python3" };
            log(&format!("Scanning ELF dependencies (Mixed + {})...", interp_pkg));
            let needed = scan_needed_libs(installed_dir)?;
            let mut deps: BTreeSet<String> = BTreeSet::new();
            for lib in &needed {
                if let Some(pkg) = soname_to_nixpkg(lib) { deps.insert(pkg.to_string()); }
            }
            let deps_vec: Vec<String> = deps.into_iter().collect();
            log(&format!("  Found {} library dependencies", deps_vec.len()));
            let gpu_mode = detect_gpu_mode(installed_dir);
            log(&format!("  GPU mode: {:?}", gpu_mode));
            log(&format!("Generating Nix derivation (autoPatchelfHook + makeWrapper + {})...", interp_pkg));
            render_mixed_package_nix(pname, installed_dir, &bins, interp_pkg, &deps_vec, &gpu_mode)
        }
        _ => {
            log("Scanning ELF dependencies...");
            let needed = scan_needed_libs(installed_dir)?;
            let mut deps: BTreeSet<String> = BTreeSet::new();
            let mut unmapped: Vec<String> = Vec::new();
            for lib in &needed {
                if let Some(pkg) = soname_to_nixpkg(lib) {
                    deps.insert(pkg.to_string());
                } else {
                    unmapped.push(lib.clone());
                }
            }
            let deps_vec: Vec<String> = deps.into_iter().collect();
            if !deps_vec.is_empty() {
                log(&format!("  Found {} library dependencies", deps_vec.len()));
            }
            if !unmapped.is_empty() {
                log(&format!("  {} libs unmapped (autoPatchelfHook will try)", unmapped.len()));
            }
            let gpu_mode = detect_gpu_mode(installed_dir);
            log(&format!("  GPU mode: {:?}", gpu_mode));
            log("Generating Nix derivation (autoPatchelfHook)...");
            render_tarball_package_nix(pname, installed_dir, &bins, &deps_vec, &gpu_mode)
        }
    };

    let flake_nix = render_flake_nix();

    fs::write(out_dir.join("package.nix"), &package_nix)?;
    fs::write(out_dir.join("flake.nix"), &flake_nix)?;

    // Hints file for later removal
    let install_ref = format!(
        "path:{}#default",
        out_dir.canonicalize().unwrap_or(out_dir.clone()).display()
    );
    let hint = format!(
        "pname: {pname}\ntype: {:?}\nsource_dir: {}\ninstall_ref: {install_ref}\n",
        runtime, installed_dir.display()
    );
    fs::write(out_dir.join("KPM_NIXIFY.txt"), &hint)?;

    run_nix_profile_add(&install_ref, &out_dir, &log)
}

/// Nixify an AppImage file.
///
/// 1. Copies the AppImage to `~/.kpm-nix/<name>/source.AppImage`
/// 2. Extracts it temporarily and scans ELF binaries for NEEDED libs
/// 3. Generates `package.nix` with detected `extraPkgs`
/// 4. Generates `flake.nix`
/// 5. Runs `nix profile add`
pub fn nixify_appimage(
    appimage_path: &Path,
    pname: &str,
    log_fn: Option<&dyn Fn(&str)>,
) -> Result<bool> {
    let log = |msg: &str| {
        if let Some(f) = log_fn {
            f(msg);
        }
    };

    let attr = sanitize_attr_name(pname);

    let out_dir = directories::BaseDirs::new()
        .context("Could not determine home directory")?
        .home_dir()
        .join(".kpm-nix")
        .join(&attr);
    fs::create_dir_all(&out_dir)?;

    // Copy AppImage
    let dest = out_dir.join("source.AppImage");
    log(&format!("Copying AppImage to {}...", out_dir.display()));
    fs::copy(appimage_path, &dest)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dest)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest, perms)?;
    }

    // Extract AppImage to scan its ELF binaries
    log("Extracting AppImage to scan dependencies...");
    let extract_dir = out_dir.join("_scan_tmp");
    let _ = fs::remove_dir_all(&extract_dir);
    fs::create_dir_all(&extract_dir)?;

    let extract_result = std::process::Command::new(&dest)
        .arg("--appimage-extract")
        .current_dir(&extract_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    let squashfs_root = extract_dir.join("squashfs-root");

    let (deps_vec, extras) = if extract_result.is_ok() && squashfs_root.exists() {
        log("Scanning extracted AppImage for library dependencies...");
        let needed = scan_needed_libs(&squashfs_root).unwrap_or_default();
        let mut deps: BTreeSet<String> = BTreeSet::new();
        let mut unmapped = 0u32;
        for lib in &needed {
            if let Some(pkg) = soname_to_nixpkg(lib) {
                deps.insert(pkg.to_string());
            } else {
                unmapped += 1;
            }
        }
        let deps_vec: Vec<String> = deps.into_iter().collect();
        // Extraction-based extras + raw scan merged for maximum coverage
        let mut extras = detect_appimage_extras(&squashfs_root);
        for e in scan_appimage_raw(&dest) {
            if !extras.contains(&e) { extras.push(e); }
        }
        log(&format!("  Found {} runtime deps, {} unmapped, {} dlopen extras",
            deps_vec.len(), unmapped, extras.len()));
        (deps_vec, extras)
    } else {
        // Extraction failed (no FUSE?) — fall back to raw byte scan
        log("  AppImage extraction failed, falling back to raw byte scan...");
        let extras = scan_appimage_raw(&dest);
        log(&format!("  Raw scan found {} extra deps", extras.len()));
        (vec![], extras)
    };

    // Clean up extracted files
    let _ = fs::remove_dir_all(&extract_dir);

    log("Generating AppImage Nix derivation (appimageTools.wrapType2)...");
    let package_nix = render_appimage_package_nix(pname, &deps_vec, &extras);
    let flake_nix = render_flake_nix();

    fs::write(out_dir.join("package.nix"), &package_nix)?;
    fs::write(out_dir.join("flake.nix"), &flake_nix)?;

    let install_ref = format!(
        "path:{}#default",
        out_dir.canonicalize().unwrap_or(out_dir.clone()).display()
    );
    let hint = format!(
        "pname: {pname}\ntype: appimage\nsource: {}\ninstall_ref: {install_ref}\n",
        dest.display()
    );
    fs::write(out_dir.join("KPM_NIXIFY.txt"), &hint)?;

    run_nix_profile_add(&install_ref, &out_dir, &log)
}

/// Run `nix profile add` with the proper experimental features flags.
fn run_nix_profile_add(
    install_ref: &str,
    out_dir: &Path,
    log: &dyn Fn(&str),
) -> Result<bool> {
    log("Building and installing Nix package (this may take a moment)...");

    let output = std::process::Command::new("nix")
        .arg("--extra-experimental-features")
        .arg("nix-command flakes")
        .arg("profile")
        .arg("add")
        .arg("--impure")
        .arg(install_ref)
        .output()
        .context("Failed to run `nix profile add`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let code = output.status.code().unwrap_or(-1);

        let log_path = out_dir.join("KPM_NIXIFY_ERROR.log");
        let body = format!(
            "command: nix profile add --impure {install_ref}\nexit_code: {code}\n\nstdout:\n{stdout}\n\nstderr:\n{stderr}"
        );
        let _ = fs::write(&log_path, &body);

        log(&format!("Nix build failed (code {}). Log: {}", code, log_path.display()));
        return Ok(false);
    }

    log("Nix package installed successfully!");
    Ok(true)
}

/// Remove a nix-installed package by its attr name.
pub fn nix_profile_remove(attr_name: &str) -> Result<bool> {
    let out_dir = directories::BaseDirs::new()
        .context("Could not determine home directory")?
        .home_dir()
        .join(".kpm-nix")
        .join(attr_name);

    // Find the exact profile Name by parsing `nix profile list` output.
    let profile_name: Option<String> = std::process::Command::new("nix")
        .args(["--extra-experimental-features", "nix-command flakes", "profile", "list"])
        .output()
        .ok()
        .and_then(|out| {
            let text = String::from_utf8_lossy(&out.stdout).to_string();
            let attr_lower = attr_name.to_lowercase();
            let mut current_name: Option<String> = None;
            for line in text.lines() {
                let line = line.trim();
                if let Some(n) = line.strip_prefix("Name:") {
                    current_name = Some(n.trim().to_string());
                } else if line.is_empty() {
                    current_name = None;
                }
                if let Some(ref name) = current_name {
                    let name_lower = name.to_lowercase();
                    if name_lower == attr_lower
                        || name_lower.replace('-', "_") == attr_lower
                        || attr_lower.replace('-', "_") == name_lower.replace('-', "_")
                    {
                        return Some(name.clone());
                    }
                }
                if (line.contains(".kpm-nix") || line.contains(".pkm-nix")) && line.contains(attr_name) {
                    if let Some(ref name) = current_name {
                        return Some(name.clone());
                    }
                }
            }
            None
        });

    let result = if let Some(ref name) = profile_name {
        std::process::Command::new("nix")
            .args(["--extra-experimental-features", "nix-command flakes", "profile", "remove", name])
            .output()
    } else {
        std::process::Command::new("nix")
            .args(["--extra-experimental-features", "nix-command flakes", "profile", "remove", attr_name])
            .output()
    };

    // Clean up local files regardless
    let _ = fs::remove_dir_all(&out_dir);

    match result {
        Ok(output) => Ok(output.status.success()),
        Err(_) => Ok(false),
    }
}

/// List all nix-installed package names (from ~/.kpm-nix/).
pub fn list_nix_packages() -> Vec<String> {
    let nix_dir = match directories::BaseDirs::new() {
        Some(b) => b.home_dir().join(".kpm-nix"),
        None => return vec![],
    };
    if !nix_dir.exists() {
        return vec![];
    }
    let mut names = Vec::new();
    if let Ok(entries) = fs::read_dir(&nix_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if entry.path().join("KPM_NIXIFY.txt").exists() {
                    names.push(entry.file_name().to_string_lossy().to_string());
                }
            }
        }
    }
    names.sort();
    names
}
