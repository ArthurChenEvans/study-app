{
  description = "Study App: write MCQs beside your sources and quiz yourself (Tauri 2, Svelte 5, Rust)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      lib = pkgs.lib;
      pnpm = pkgs.pnpm_10;
    in {
      # `nix build` / `nix run`, or add it to environment.systemPackages.
      packages.${system}.default = pkgs.rustPlatform.buildRustPackage (finalAttrs: {
        pname = "study-app";
        version = "0.1.0";
        src = lib.cleanSource ./.;

        cargoLock.lockFile = ./Cargo.lock;
        buildAndTestSubdir = "src-tauri";

        pnpmDeps = pkgs.fetchPnpmDeps {
          inherit (finalAttrs) pname version src;
          inherit pnpm;
          fetcherVersion = 3;
          # Changes whenever pnpm-lock.yaml changes. If the build stops with a
          # hash mismatch, copy the "got: sha256-..." value in here.
          hash = "sha256-5p/Qv9XxrDA1wt4OeZnIoQ+Sxps3jSSPOd552PxrnLA=";
        };

        nativeBuildInputs = with pkgs; [
          cargo-tauri.hook
          pnpmConfigHook
          pnpm
          nodejs_22
          pkg-config
          wrapGAppsHook3
        ];

        buildInputs = with pkgs; [
          openssl
          webkitgtk_4_1
          gtk3
          libsoup_3
          glib-networking
          librsvg
          gsettings-desktop-schemas
        ];

        # The Tauri hook builds the .deb bundle and installs from it.
        tauriBundleType = "deb";

        # Tests run with `cargo test` and `pnpm test` in the dev shell; the
        # conversion tests need LibreOffice and a writable home.
        doCheck = false;

        # PowerPoint previews convert through LibreOffice, so put it on the app's PATH.
        preFixup = ''
          gappsWrapperArgs+=(--prefix PATH : ${lib.makeBinPath [ pkgs.libreoffice ]})
        '';

        meta = {
          description = "Write multiple-choice questions beside your sources and quiz yourself";
          homepage = "https://github.com/ArthurChenEvans/study-app";
          platforms = [ system ];
          mainProgram = "study-app";
        };
      });

      apps.${system}.default = {
        type = "app";
        program = "${self.packages.${system}.default}/bin/study-app";
      };

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
