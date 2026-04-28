{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    cargo
    rustc
    rustfmt
    clippy
    pkg-config
  ];

  buildInputs = with pkgs; [
    openssl
    zlib
  ];

  # Helpful for crates that use openssl-sys
  shellHook = ''
    export OPENSSL_NO_VENDOR=1
  '';
}

