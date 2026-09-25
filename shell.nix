flake:
{ pkgs, ... }:
pkgs.mkShell {
  packages = with pkgs; [
    nixd
    statix
    deadnix
    nixfmt

    rustc
    rustfmt
    cargo
    clippy
    rust-analyzer
    cargo-watch

    # Other packages here
    # openssl
    # libressl
    # ...
  ];

  RUST_BACKTRACE = "full";
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

  shellHook = ''
    # Extra steps to do while activating development shell
  '';
}
