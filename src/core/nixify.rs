use anyhow::{Context, Result};
use base64::Engine;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

fn score_asset_for_x86_64_linux(name: &str) -> i32 {
    // Higher is better.
    let n = name.to_lowercase();
    let mut score = 0;
    if n.contains("linux") {
        score += 50;
    }
    if n.contains("x86_64") || n.contains("amd64") {
        score += 50;
    }
    // Penalize common non-x86_64 markers
    if n.contains("aarch64") || n.contains("arm64") {
        score -= 80;
    }
    if n.contains("armv7") || n.contains("armv6") || n.contains("armhf") || n.contains("arm") {
        score -= 40;
    }
    if n.contains("i386") || n.contains("i686") || n.contains("x86") {
        // i686 builds exist sometimes; mild penalty vs x86_64.
        score -= 10;
    }
    // Prefer archives over source
    if n.ends_with(".tar.gz") || n.ends_with(".tar.xz") || n.ends_with(".tar.bz2") || n.ends_with(".zip") {
        score += 10;
    }
    // Avoid "source" or "src"
    if n.contains("source") || n.contains("src") {
        score -= 20;
    }
    score
}

fn assert_x86_64_elf(path: &Path) -> Result<()> {
    let bytes = fs::read(path)?;
    if !is_elf(&bytes) {
        return Ok(());
    }
    if let Ok(elf) = goblin::elf::Elf::parse(&bytes) {
        // EM_X86_64 = 62
        if elf.header.e_machine != goblin::elf::header::EM_X86_64 {
            anyhow::bail!(
                "Downloaded binary is not x86_64 (e_machine={}). You likely got an ARM build. Re-run nixify and ensure the selected asset is linux-x86_64/amd64.",
                elf.header.e_machine
            );
        }
    }
    Ok(())
}

fn infer_name_from_path(p: &Path) -> String {
    let file_name = p
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    file_name
        .replace(".tar.gz", "")
        .replace(".tar.xz", "")
        .replace(".tar.bz2", "")
        .replace(".pkg.tar.xz", "")
        .replace(".zip", "")
}

fn sri_sha256(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let mut h = Sha256::new();
    h.update(&bytes);
    let digest = h.finalize();
    let b64 = base64::engine::general_purpose::STANDARD.encode(digest);
    Ok(format!("sha256-{}", b64))
}

fn is_elf(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0] == 0x7f && bytes[1] == b'E' && bytes[2] == b'L' && bytes[3] == b'F'
}

fn needed_libs_for_file(path: &Path) -> Result<Vec<String>> {
    let bytes = fs::read(path)?;
    if !is_elf(&bytes) {
        return Ok(vec![]);
    }
    match goblin::elf::Elf::parse(&bytes) {
        Ok(elf) => Ok(elf.libraries.into_iter().map(|s| s.to_string()).collect()),
        Err(_) => Ok(vec![]),
    }
}

fn suggest_nixpkgs_for_soname(soname: &str) -> Option<&'static str> {
    // Best-effort mapping. Unknown libs are left unmapped so the user can adjust buildInputs.
    let s = soname;
    Some(match s {
        "libstdc++.so.6" | "libgcc_s.so.1" => "stdenv.cc.cc",
        "libz.so.1" => "zlib",
        "libbz2.so.1.0" => "bzip2",
        "liblzma.so.5" => "xz",
        "libssl.so.1.1" | "libcrypto.so.1.1" => "openssl_1_1",
        "libssl.so.3" | "libcrypto.so.3" => "openssl",
        "libasound.so.2" => "alsa-lib",
        "libudev.so.1" => "eudev",
        "libdbus-1.so.3" => "dbus",
        "libcurl.so.4" => "curl",
        // X11 (prefer non-xorg aliases; xorg.* is deprecated in nixpkgs)
        "libX11.so.6" => "libx11",
        "libXext.so.6" => "libxext",
        "libXrender.so.1" => "libxrender",
        "libXi.so.6" => "libxi",
        "libXfixes.so.3" => "libxfixes",
        "libXrandr.so.2" => "libxrandr",
        "libxcb.so.1" => "libxcb",
        "libXcomposite.so.1" => "libxcomposite",
        "libXdamage.so.1" => "libxdamage",
        "libXcursor.so.1" => "libxcursor",
        "libXinerama.so.1" => "libxinerama",
        "libXtst.so.6" => "libXtst",
        "libXss.so.1" => "libXScrnSaver",
        "libwayland-client.so.0" => "wayland",
        "libwayland-cursor.so.0" => "wayland",
        "libwayland-egl.so.1" => "wayland",
        "libGL.so.1" => "libGL",
        "libGLX.so.0" => "libglvnd",
        "libEGL.so.1" => "libglvnd",
        "libGLESv2.so.2" => "libglvnd",

        // GTK / GNOME stack (common for Electron builds)
        "libglib-2.0.so.0" => "glib",
        "libgobject-2.0.so.0" => "glib",
        "libgio-2.0.so.0" => "glib",
        "libgmodule-2.0.so.0" => "glib",
        "libgthread-2.0.so.0" => "glib",
        "libgtk-3.so.0" => "gtk3",
        "libgdk-3.so.0" => "gtk3",
        "libatk-1.0.so.0" => "atk",
        "libatk-bridge-2.0.so.0" => "atk",
        "libatspi.so.0" => "at-spi2-core",
        "libpango-1.0.so.0" => "pango",
        "libpangocairo-1.0.so.0" => "pango",
        "libpangoft2-1.0.so.0" => "pango",
        "libcairo.so.2" => "cairo",
        "libgdk_pixbuf-2.0.so.0" => "gdk-pixbuf",

        // Chromium/Electron runtime deps
        "libnspr4.so" => "nspr",
        "libnss3.so" => "nss",
        "libnssutil3.so" => "nss",
        "libsmime3.so" => "nss",
        "libcups.so.2" => "cups",
        "libgbm.so.1" => "mesa",
        "libexpat.so.1" => "expat",
        "libxkbcommon.so.0" => "libxkbcommon",

        _ => return None,
    })
}

fn pick_link_bins(extracted_root: &Path) -> Vec<String> {
    // Prefer usr/bin and bin, but also include root-level executables (common in app bundles).
    let mut rels = BTreeSet::new();
    for candidate_dir in ["usr/bin", "bin", "."] {
        let p = extracted_root.join(candidate_dir);
        if let Ok(rd) = fs::read_dir(&p) {
            for entry in rd.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(meta) = entry.metadata() {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            if meta.permissions().mode() & 0o111 != 0 {
                                if let Ok(r) = path.strip_prefix(extracted_root) {
                                    rels.insert(r.to_string_lossy().to_string());
                                }
                            }
                        }
                        #[cfg(not(unix))]
                        {
                            if let Ok(r) = path.strip_prefix(extracted_root) {
                                rels.insert(r.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    rels.into_iter().collect()
}

fn render_default_nix(
    pname: &str,
    version: &str,
    url: Option<&str>,
    sha256_sri: Option<&str>,
    impure_src_path: Option<&Path>,
    linked_bins: &[String],
    suggested_inputs: &[String],
    unmapped_libs: &[String],
    extra_flags: &str,
) -> String {
    let mut args = vec![
        "stdenv",
        "lib",
        "autoPatchelfHook",
        "fetchurl",
        "pkgs",
    ];
    // Add attr args for suggested inputs, keeping dotted attrs as-is (e.g. xorg.libX11).
    // We don't try to add args for dotted attrs; they will be referenced via pkgs in flake instead.
    // So only include "plain" attrs here.
    let mut plain_args = BTreeSet::new();
    for s in suggested_inputs {
        if !s.contains('.') {
            plain_args.insert(s.as_str());
        }
    }
    for a in plain_args {
        args.push(a);
    }

    let mut build_inputs = Vec::new();
    for s in suggested_inputs {
        if s.contains('.') {
            build_inputs.push(format!("pkgs.{}", s));
        } else {
            build_inputs.push(s.clone());
        }
    }

    let src_expr = if let Some(p) = impure_src_path {
        // Nix paths must be path literals (not strings). Require absolute paths here.
        let abs = if p.is_absolute() {
            p.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(p)
        };
        format!("builtins.path {{ path = {}; }}", abs.display())
    } else {
        let url = url.unwrap_or("");
        let sha256 = sha256_sri.unwrap_or("sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
        format!(
            "fetchurl {{\n    url = \"{}\";\n    sha256 = \"{}\";\n  }}",
            url, sha256
        )
    };

    let mut link_lines = String::new();
    if !linked_bins.is_empty() {
        for rel in linked_bins {
            let base = Path::new(rel)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            link_lines.push_str(&format!("  ln -s \"$out/{}\" \"$out/bin/{}\"\n", rel, base));
        }
    }

    let mut notes = String::new();
    if !unmapped_libs.is_empty() {
        notes.push_str("  # Unmapped ELF dependencies detected (add to buildInputs if autoPatchelf fails):\n");
        for l in unmapped_libs {
            notes.push_str(&format!("  # - {}\n", l));
        }
    }

    // Avoid `{`/`}` escaping issues in `format!` by using a template + replace.
    let template = r#"{
  __ARGS__
}:

stdenv.mkDerivation rec {
  pname = "__PNAME__";
  version = "__VERSION__";

  src = __SRC_EXPR__;

  nativeBuildInputs = [
    autoPatchelfHook
  ];

  buildInputs = [
__BUILD_INPUTS__
  ];

  sourceRoot = ".";

  installPhase = ''
    runHook preInstall

    # Some archives unpack into a single top-level directory (e.g. vesktop-1.6.5/).
    # Auto-detect and copy from the effective root.
    shopt -s nullglob
    root="."
    entries=(./*)
    if [ "''${#entries[@]}" -eq 1 ] && [ -d "''${entries[0]}" ]; then
      root="''${entries[0]}"
    fi

    mkdir -p "$out"
    cp -a "$root"/* "$out/" 2>/dev/null || true

    mkdir -p "$out/bin"
__LINK_LINES__

    # If symlinks couldn't be created because files live under an extra root dir,
    # try again by searching one directory deep.
    for b in "$out/bin/"*; do
      if [ -L "$b" ] && [ ! -e "$b" ]; then
        rm -f "$b"
      fi
    done
    for rel in __REL_BINS__; do
      base="$(basename "$rel")"
      if [ -e "$out/$rel" ]; then
        ln -sf "$out/$rel" "$out/bin/$base"
      else
        for cand in "$out"/*/"$rel"; do
          if [ -e "$cand" ]; then
            ln -sf "$cand" "$out/bin/$base"
            break
          fi
        done
      fi
    done

    # Fix common Electron/Chromium dlopen() sonames when the bundle ships only the unversioned .so
    # (e.g. libEGL.so present but libEGL.so.1 missing).
    for d in "$out" "$out"/*; do
      [ -d "$d" ] || continue
      if [ -f "$d/libEGL.so" ] && [ ! -e "$d/libEGL.so.1" ]; then
        ln -s "libEGL.so" "$d/libEGL.so.1"
      fi
      if [ -f "$d/libGLESv2.so" ] && [ ! -e "$d/libGLESv2.so.2" ]; then
        ln -s "libGLESv2.so" "$d/libGLESv2.so.2"
      fi
      if [ -f "$d/libvulkan.so.1" ] && [ ! -e "$d/libvulkan.so" ]; then
        # Some apps dlopen libvulkan.so; Nix usually provides it via vulkan-loader, but this helps bundles.
        ln -s "libvulkan.so.1" "$d/libvulkan.so" || true
      fi
    done

    # Avoid makeWrapper/wrapProgram because it creates a fixed "$out/env-vars" file,
    # which conflicts when installing multiple nixified packages into the same profile.
    # Instead, generate per-command wrapper scripts in $out/bin.
    runtimeLibPath="${lib.makeLibraryPath buildInputs}:$out"
    for d in "$out"/*; do
      [ -d "$d" ] || continue
      runtimeLibPath="$runtimeLibPath:$d"
    done

    for rel in __REL_BINS__; do
      base="$(basename "$rel")"
      case "$base" in
        *.so|*.so.*) continue ;;
      esac

      target=""
      if [ -x "$out/$rel" ]; then
        target="$out/$rel"
      else
        for cand in "$out"/*/"$rel"; do
          if [ -x "$cand" ]; then
            target="$cand"
            break
          fi
        done
      fi

      if [ -n "$target" ]; then
        # Replace symlink (if any) with a wrapper.
        rm -f "$out/bin/$base"
        cat > "$out/bin/$base" <<EOF
#!${pkgs.runtimeShell}
if [ -n "$LD_LIBRARY_PATH" ]; then
  export LD_LIBRARY_PATH="$runtimeLibPath:$LD_LIBRARY_PATH"
else
  export LD_LIBRARY_PATH="$runtimeLibPath"
fi
extraFlags="__EXTRA_FLAGS__"
if [ -n "$extraFlags" ]; then
  # shellcheck disable=SC2086
  exec "$target" $extraFlags "\$@"
else
  exec "$target" "\$@"
fi
EOF
        chmod +x "$out/bin/$base"
      fi
    done
    runHook postInstall
  '';

__NOTES__
  meta = {
    description = "Packaged by Kore Package Manager nixify";
    platforms = [ "x86_64-linux" ];
  };
}
"#;

    let args_joined = args.join(",\n  ");
    let build_inputs_joined = build_inputs
        .into_iter()
        .map(|x| format!("    {}", x))
        .collect::<Vec<_>>()
        .join("\n");

    let rel_bins_sh = if linked_bins.is_empty() {
        "\"\"".to_string()
    } else {
        linked_bins
            .iter()
            .map(|r| format!("{:?}", r))
            .collect::<Vec<_>>()
            .join(" ")
    };

    template
        .replace("__ARGS__", &args_joined)
        .replace("__PNAME__", pname)
        .replace("__VERSION__", version)
        .replace("__SRC_EXPR__", &src_expr)
        .replace("__BUILD_INPUTS__", &build_inputs_joined)
        .replace("__LINK_LINES__", link_lines.trim_end_matches('\n'))
        .replace("__NOTES__", notes.trim_end_matches('\n'))
        .replace("__REL_BINS__", &rel_bins_sh)
        .replace("__EXTRA_FLAGS__", extra_flags)
}

fn render_flake_nix(pname: &str) -> String {
    let template = r#"{
  description = "Generated by kpm nixify";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in
    {
      packages.${system} = {
        "__PNAME__" = pkgs.callPackage ./default.nix { inherit pkgs; };
        default = self.packages.${system}."__PNAME__";
      };
    };
}
"#;
    template.replace("__PNAME__", pname)
}

fn extract_archive_to_dir(archive: &Path, target: &Path) -> Result<()> {
    if target.exists() {
        let _ = fs::remove_dir_all(target);
    }
    fs::create_dir_all(target)?;

    let file_name = archive.file_name().unwrap_or_default().to_string_lossy();
    let is_zip = file_name.ends_with(".zip");

    if is_zip {
        let status = std::process::Command::new("unzip")
            .args(["-q", archive.to_str().unwrap(), "-d", target.to_str().unwrap()])
            .status()
            .context("Failed to run unzip")?;
        anyhow::ensure!(status.success(), "unzip failed");
        return Ok(());
    }

    // Decide if we need to strip a single top-level directory
    let output = std::process::Command::new("tar")
        .arg("-tf")
        .arg(archive)
        .output()
        .context("Failed to list tar contents")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut first_components = std::collections::HashSet::new();
    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('/').collect();
        if !parts.is_empty() {
            first_components.insert(parts[0]);
        }
    }

    let mut args = vec!["-xf", archive.to_str().unwrap(), "-C", target.to_str().unwrap()];
    if first_components.len() == 1 {
        args.push("--strip-components=1");
    }

    let status = std::process::Command::new("tar")
        .args(&args)
        .status()
        .context("Failed to run tar")?;
    anyhow::ensure!(status.success(), "tar extraction failed");
    Ok(())
}

fn scan_needed_libs(root: &Path) -> Result<BTreeSet<String>> {
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
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let looks_like_elf = name.ends_with(".so")
            || name.contains(".so.")
            || name.is_empty()
            || entry.metadata().map(|m| m.len() > 0).unwrap_or(false);
        if !looks_like_elf {
            continue;
        }
        if let Ok(needed) = needed_libs_for_file(p) {
            for n in needed {
                libs.insert(n);
            }
        }
    }
    Ok(libs)
}

fn looks_like_electron_bundle(root: &Path) -> bool {
    let markers = [
        "chrome-sandbox",
        "chrome_crashpad_handler",
        "resources.pak",
        "icudtl.dat",
        "snapshot_blob.bin",
        "v8_context_snapshot.bin",
    ];

    for m in markers {
        if root.join(m).exists() {
            return true;
        }
    }
    if let Ok(rd) = fs::read_dir(root) {
        for e in rd.flatten() {
            let p = e.path();
            if !p.is_dir() {
                continue;
            }
            for m in markers {
                if p.join(m).exists() {
                    return true;
                }
            }
        }
    }
    false
}

pub async fn nixify(
    source: &str,
    out_dir: Option<&str>,
    pname_override: Option<&str>,
    version_override: Option<&str>,
    impure_local: bool,
) -> Result<()> {
    let mut archive_path = PathBuf::from(source);
    let local_archive_dir = if archive_path.exists() {
        archive_path.parent().map(|p| p.to_path_buf())
    } else {
        None
    };
    let mut url: Option<String> = None;
    let mut downloaded = false;
    let mut inferred_name: Option<String> = None;

    if !archive_path.exists() {
        // Try resolve from repositories.
        let config = crate::config::Config::new();
        if let Some(resolved) = crate::core::install::resolve_source(&config, source).await? {
            inferred_name = resolved.repo_package_name.or(resolved.repo_name);
            let resolved_url = if resolved.is_git {
                let assets = crate::core::download::get_latest_release_assets(&resolved.url).await?;
                anyhow::ensure!(!assets.is_empty(), "No suitable tarball assets found in the latest release");
                let mut best = &assets[0];
                let mut best_score = score_asset_for_x86_64_linux(&best.name);
                for a in &assets[1..] {
                    let s = score_asset_for_x86_64_linux(&a.name);
                    if s > best_score {
                        best = a;
                        best_score = s;
                    }
                }
                best.browser_download_url.clone()
            } else {
                crate::core::dynamic_links::resolve_dynamic_url(&resolved.url).await?
            };
            url = Some(resolved_url.clone());

            let tmp_dir = std::env::temp_dir().join("kpm_nixify_downloads");
            fs::create_dir_all(&tmp_dir)?;
            archive_path = crate::core::download::download_file(&resolved_url, &tmp_dir, None).await?;
            downloaded = true;
        } else {
            // Treat as direct URL.
            let resolved_url = crate::core::dynamic_links::resolve_dynamic_url(source).await.unwrap_or_else(|_| source.to_string());
            url = Some(resolved_url.clone());
            let tmp_dir = std::env::temp_dir().join("kpm_nixify_downloads");
            fs::create_dir_all(&tmp_dir)?;
            archive_path = crate::core::download::download_file(&resolved_url, &tmp_dir, None).await?;
            downloaded = true;
        }
    }

    let pname = pname_override
        .map(|s| s.to_string())
        .or(inferred_name)
        .unwrap_or_else(|| infer_name_from_path(&archive_path));
    let version = version_override.unwrap_or("0.0.0").to_string();

    let out_dir = out_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            // If the user provided a local archive path, write next to it by default.
            if let Some(parent) = &local_archive_dir {
                parent.join(format!("nixify-{}", pname))
            } else {
                PathBuf::from(format!("./nixify-{}", pname))
            }
        });
    fs::create_dir_all(&out_dir)?;

    let sha256 = if downloaded { Some(sri_sha256(&archive_path)?) } else { None };

    // Extract to temp and scan ELF deps.
    let extract_dir = std::env::temp_dir().join(format!("kpm_nixify_extract_{}", pname));
    extract_archive_to_dir(&archive_path, &extract_dir)?;
    let needed = scan_needed_libs(&extract_dir)?;

    let mut suggested: BTreeSet<String> = BTreeSet::new();
    let mut unmapped: BTreeSet<String> = BTreeSet::new();
    for lib in needed {
        if let Some(attr) = suggest_nixpkgs_for_soname(&lib) {
            suggested.insert(attr.to_string());
        } else {
            unmapped.insert(lib);
        }
    }

    // De-dup + stabilize order
    let suggested_vec: Vec<String> = suggested.into_iter().collect();
    let unmapped_vec: Vec<String> = unmapped.into_iter().collect();
    let linked_bins = pick_link_bins(&extract_dir);

    let extra_flags = if looks_like_electron_bundle(&extract_dir) {
        "--disable-gpu-sandbox --ozone-platform-hint=auto"
    } else {
        ""
    };

    // If we have a likely main binary at the root or bundle dir, sanity check arch.
    // (This won't catch everything, but prevents obvious ARM-on-x86_64 mistakes.)
    if linked_bins.len() == 1 {
        let p = extract_dir.join(&linked_bins[0]);
        let _ = assert_x86_64_elf(&p);
    } else {
        // Prefer checking a binary named like pname if present.
        for rel in &linked_bins {
            let base = Path::new(rel).file_name().and_then(|s| s.to_str()).unwrap_or("");
            if base == pname {
                let p = extract_dir.join(rel);
                let _ = assert_x86_64_elf(&p);
                break;
            }
        }
    }

    let impure_src_path = if !downloaded && impure_local { Some(archive_path.as_path()) } else { None };
    let default_nix = render_default_nix(
        &pname,
        &version,
        url.as_deref(),
        sha256.as_deref(),
        impure_src_path,
        &linked_bins,
        &suggested_vec,
        &unmapped_vec,
        extra_flags,
    );
    let flake_nix = render_flake_nix(&pname);

    fs::write(out_dir.join("default.nix"), default_nix)?;
    fs::write(out_dir.join("flake.nix"), flake_nix)?;

    // Optional: write a small hints file.
    let mut hints = BTreeMap::new();
    hints.insert("pname", pname.clone());
    hints.insert("version", version.clone());
    if let Some(u) = &url {
        hints.insert("url", u.clone());
    }
    if let Some(s) = &sha256 {
        hints.insert("sha256_sri", s.clone());
    }
    let mut hint_txt = String::new();
    for (k, v) in hints {
        hint_txt.push_str(&format!("{}: {}\n", k, v));
    }
    hint_txt.push_str("\nInstall:\n");
    // Use `path:` to avoid Git tracking issues when inside a repo (flakes only see tracked files).
    // Add `--impure` only when needed (local archive via builtins.path or environment-based allowances).
    let install_ref = format!("path:{}#\\\"{}\\\"", out_dir.canonicalize().unwrap_or(out_dir.clone()).display(), pname);
    if impure_local {
        hint_txt.push_str(&format!("  nix profile add --impure \"{}\"\n", install_ref));
    } else {
        hint_txt.push_str(&format!("  nix profile add \"{}\"\n", install_ref));
    }
    fs::write(out_dir.join("KPM_NIXIFY.txt"), hint_txt)?;

    // Cleanup downloaded archive.
    if downloaded {
        let _ = fs::remove_file(&archive_path);
    }
    let _ = fs::remove_dir_all(&extract_dir);

    Ok(())
}

