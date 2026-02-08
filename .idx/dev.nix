{ pkgs, ... }: {
  # Nutze stable oder unstable (unstable hat oft aktuelleres Rust)
  channel = "stable-23.11";

  packages = [
    pkgs.rustc
    pkgs.cargo
    pkgs.rustfmt
    pkgs.rust-analyzer
    # KRITISCH: gcc liefert den Linker (cc), ohne den Proc Macros fehlschlagen
    pkgs.gcc
  ];

  # Notwendig, damit rust-analyzer die Standardbibliothek findet
  env = {
    RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
  };

  idx = {
    extensions = [
      "rust-lang.rust-analyzer"
      "tamasfe.even-better-toml"
    ];
  };
}
