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
          webkitgtk_4_1 gtk3 libsoup_3 glib-networking librsvg gsettings-desktop-schemas
          # PowerPoint to PDF conversion for the source preview
          libreoffice
        ];
        shellHook = ''
          export GIO_MODULE_DIR=${pkgs.glib-networking}/lib/gio/modules/
          # GTK dialogs (the file chooser behind "Open file") need their GSettings
          # schemas on XDG_DATA_DIRS, which NixOS doesn't set up outside a wrapped app.
          export XDG_DATA_DIRS=${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}:$XDG_DATA_DIRS
        '';
      };
    };
}
