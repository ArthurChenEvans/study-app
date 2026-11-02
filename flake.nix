{
  description = "Study App dev shell (Tauri 2, Svelte 5, Rust)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in {
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          cargo rustc rustfmt clippy rust-analyzer
          nodejs_22 pnpm
          pkg-config openssl
          # Tauri 2 on Linux
          webkitgtk_4_1 gtk3 libsoup_3 glib-networking librsvg
          # PowerPoint to PDF conversion for the source preview
          libreoffice
        ];
        shellHook = ''
          export GIO_MODULE_DIR=${pkgs.glib-networking}/lib/gio/modules/
        '';
      };
    };
}
