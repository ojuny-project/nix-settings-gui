{
  description = "nix-settings-gui: a comment-driven settings GUI for Nix";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    tinted-theming = {
      url = "github:tinted-theming/schemes";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      tinted-theming,
      ...
    }:
    let
      binaryInfo =
        if builtins.pathExists ./binary-url.json then
          builtins.fromJSON (builtins.readFile ./binary-url.json)
        else
          null;

      desktopPostInstall = ''
        install -Dm644 ${./org.nix_settings_gui.NixSettingsGui.desktop} "$out/share/applications/org.nix_settings_gui.NixSettingsGui.desktop"
      '';

      mkBinary =
        {
          system,
          tag ? (
            if binaryInfo == null then
              throw "nix-settings-gui: binary-url.json is missing"
            else
              binaryInfo.latest
          ),
          package ? "nix-settings-gui",
        }:
        let
          release =
            if binaryInfo == null then
              throw "nix-settings-gui: binary-url.json is missing"
            else
              binaryInfo.releases.${tag}
                or (throw "nix-settings-gui: release '${tag}' is not recorded in binary-url.json");
          packageInfo =
            release.packages.${package}
              or (throw "nix-settings-gui: package '${package}' is not recorded in release '${tag}'");
          pkgs = import nixpkgs { inherit system; };
        in
        pkgs.callPackage ./nix/binary.nix {
          name = package;
          program = packageInfo.program or package;
          inherit (packageInfo) file;
          info = {
            inherit (binaryInfo) repo system;
            inherit (release) tag version;
            inherit (packageInfo) url hash;
          };
          postInstall = desktopPostInstall;
        };
    in
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        inherit (pkgs) lib;

        runtimePath = lib.makeBinPath (
          with pkgs;
          [
            nix
            nixd
            nixfmt
            wlr-randr
            curl
            git
            libxcrypt
            openssl
            sbctl
            systemd
            polkit
          ]
        );

        nativeBuildInputs = with pkgs; [
          pkg-config
          mold
          wrapGAppsHook4
        ];

        buildInputs = with pkgs; [
          gtk4
          libadwaita
          glib
        ];

        mkPackage =
          {
            withOjunyWidgets ? true,
            pname ? "nix-settings-gui",
          }:
          pkgs.rustPlatform.buildRustPackage {
            pname = pname;
            version = "0.1.0";
            src = lib.fileset.toSource {
              root = ./.;
              fileset = lib.fileset.unions [
                (lib.fileset.fileFilter (f: f.hasExt "rs") ./.)
                (lib.fileset.fileFilter (
                  f:
                  f.name == "Cargo.toml"
                  || f.name == "Cargo.lock"
                  || f.name == "org.nix_settings_gui.NixSettingsGui.desktop"
                ) ./.)
                ./assets
                ./modules
                ./src
                ./tests
              ];
            };
            cargoLock.lockFile = ./Cargo.lock;
            cargoBuildFlags = lib.optionals (!withOjunyWidgets) [ "--no-default-features" ];

            TINTED_SCHEMES_DIR = "${tinted-theming}";
            NSG_SOURCE_REVISION = self.shortRev or self.dirtyShortRev or "unknown";

            RUSTFLAGS = "-C link-arg=-fuse-ld=mold";
            inherit nativeBuildInputs buildInputs;

            doCheck = true;

            preFixup = ''
              gappsWrapperArgs+=(--prefix PATH : "${runtimePath}")
            '';

            postInstall = ''
              install -Dm644 ${./org.nix_settings_gui.NixSettingsGui.desktop} \
                "$out/share/applications/org.nix_settings_gui.NixSettingsGui.desktop"
            '';

            meta = with lib; {
              description = "A comment-driven settings GUI for Nix";
              mainProgram = "nix-settings-gui";
              license = licenses.gpl3Plus;
              platforms = platforms.linux;
            };
          };

        nix-settings-gui = mkPackage { };
        nix-settings-gui-minimal = mkPackage {
          withOjunyWidgets = false;
          pname = "nix-settings-gui-minimal";
        };
      in
      {
        packages = {
          inherit nix-settings-gui nix-settings-gui-minimal;
          default = nix-settings-gui;
        }
        // (
          if binaryInfo != null && binaryInfo.system == system && binaryInfo.latest != "" then
            {
              nix-settings-gui-binary = mkBinary { inherit system; };
            }
            // (
              if binaryInfo.releases.${binaryInfo.latest}.packages ? nix-settings-gui-minimal then
                {
                  nix-settings-gui-minimal-binary = mkBinary {
                    inherit system;
                    package = "nix-settings-gui-minimal";
                  };
                }
              else
                { }
            )
          else
            { }
        );

        apps.default = {
          type = "app";
          program = "${nix-settings-gui}/bin/nix-settings-gui";
        };

        apps.backend = {
          type = "app";
          program = "${nix-settings-gui}/bin/nix-settings-backend";
        };

        apps.ci = {
          type = "app";
          program = "${
            pkgs.writeShellApplication {
              name = "ci";
              runtimeInputs = [ pkgs.nix ];
              text = ''
                exec nix flake check "$@"
              '';
            }
          }/bin/ci";
        };

        checks.tests = nix-settings-gui;

        checks.gui = nix-settings-gui.overrideAttrs (old: {
          nativeBuildInputs = old.nativeBuildInputs ++ [
            pkgs.gnugrep
            pkgs.gnused
            pkgs.xvfb-run
          ];
          preCheck = (old.preCheck or "") + ''
            export XDG_RUNTIME_DIR="$(mktemp -d)"
            export XDG_DATA_DIRS="${pkgs.gsettings-desktop-schemas}/share:${pkgs.adwaita-icon-theme}/share:${pkgs.hicolor-icon-theme}/share:''${XDG_DATA_DIRS:-}"
          '';
          checkPhase = ''
            runHook preCheck
            tests="$(cargo test --release --locked -- --ignored --list \
              | sed -n 's/: test$//p' \
              | grep -v -e 'metadata::tests::loads_' -e 'plugin_schema::tests::parses_the_real_bar_dropdown_settings')"
            for test in $tests; do
              xvfb-run -a cargo test --release --locked -- --exact "$test" --include-ignored
            done
            runHook postCheck
          '';
        });

        checks.minimal = pkgs.runCommand "check-minimal" { } ''
          echo ${self.packages.${system}.nix-settings-gui-minimal} >/dev/null
          touch $out
        '';

        checks.default = pkgs.runCommand "check-default" { } ''
          echo ${self.packages.${system}.default} >/dev/null
          echo ${self.checks.${system}.tests} >/dev/null
          echo ${self.checks.${system}.gui} >/dev/null
          touch $out
        '';

        devShells.default = pkgs.mkShell {
          nativeBuildInputs =
            nativeBuildInputs
            ++ (with pkgs; [
              cargo
              rustc
              rustfmt
              clippy
              rust-analyzer
            ]);

          buildInputs =
            buildInputs
            ++ (with pkgs; [
              nix
              nixd
              nixfmt
              sbctl
              systemd
              polkit
            ]);

          TINTED_SCHEMES_DIR = "${tinted-theming}";
          NSG_SOURCE_REVISION = self.shortRev or self.dirtyShortRev or "unknown";
          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
        };
      }
    )
    // {
      hydraJobs = self.checks;
      lib.mkBinary = mkBinary;
      nixosModules.default = import ./modules/nixos.nix self;
      nixosModules.nix-settings-gui = import ./modules/nixos.nix self;
      homeManagerModules.default = import ./modules/home-manager.nix self;
      homeManagerModules.nix-settings-gui = import ./modules/home-manager.nix self;
    };
}
