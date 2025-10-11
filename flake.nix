{
  description = "Simple flexbox-inspired layout manager for tmux.";
  inputs = {
    nixpkgs.url = "github:NixOs/nixpkgs";
    devshell.url = "github:numtide/devshell";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rustnix = {
      url = "github:ck3mp3r/flakes/fix/rustnix-single-artifact?dir=rustnix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    flake-utils,
    devshell,
    nixpkgs,
    fenix,
    rustnix,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [devshell.overlays.default];
        pkgs = import nixpkgs {inherit system overlays;};
        toolchain = with fenix.packages.${system};
          combine [
            stable.cargo
            stable.rust-analyzer
            stable.rustc
            stable.rustfmt
            stable.clippy
            targets.aarch64-apple-darwin.stable.rust-std
            targets.aarch64-unknown-linux-musl.stable.rust-std
            targets.x86_64-apple-darwin.stable.rust-std
            targets.x86_64-unknown-linux-musl.stable.rust-std
          ];

        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        cargoLock = {lockFile = ./Cargo.lock;};

        # Install data for pre-built releases
        installData = {
          aarch64-darwin = builtins.fromJSON (builtins.readFile ./nix/data/aarch64-darwin.json);
          x86_64-darwin = builtins.fromJSON (builtins.readFile ./nix/data/x86_64-darwin.json);
          aarch64-linux = builtins.fromJSON (builtins.readFile ./nix/data/aarch64-linux.json);
          x86_64-linux = builtins.fromJSON (builtins.readFile ./nix/data/x86_64-linux.json);
        };

        # Build regular packages (no archives)
        regularPackages = rustnix.lib.rust.buildPackage {
          inherit
            cargoToml
            cargoLock
            fenix
            nixpkgs
            overlays
            pkgs
            system
            installData
            ;
          src = ./.;
          packageName = "laio";
          archiveAndHash = false;
        };

        # Build archive packages (creates archive with system name)
        archivePackages = rustnix.lib.rust.buildPackage {
          inherit
            cargoToml
            cargoLock
            fenix
            nixpkgs
            overlays
            pkgs
            system
            installData
            ;
          src = ./.;
          packageName = "archive";
          archiveAndHash = true;
        };
      in rec {
        apps = {
          default = {
            type = "app";
            program = "${packages.default}/bin/laio";
          };
        };

        packages =
          regularPackages
          // archivePackages
          // {
            
            tmux-mcp-tools = let
              cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
            in
              pkgs.stdenvNoCC.mkDerivation {
                pname = "tmux-mcp-tools";
                version = cargoToml.package.version;
                src = ./mcp-tools;

                dontBuild = true;
                dontConfigure = true;

                installPhase = ''
                  runHook preInstall

                  mkdir -p $out/share/nushell/mcp-tools/tmux
                  cp tmux.nu $out/share/nushell/mcp-tools/tmux/tmux.nu

                  runHook postInstall
                '';

                meta = with pkgs.lib; {
                  description = "MCP tools for tmux session management via nu-mcp";
                  homepage = "https://github.com/ck3mp3r/laio-cli";
                  license = licenses.mit;
                  maintainers = [];
                  platforms = platforms.all;
                };
              };
          };

        devShells.default = pkgs.devshell.mkShell {
          packages = [toolchain];
          imports = [(pkgs.devshell.importTOML ./devshell.toml) "${devshell}/extra/git/hooks.nix"];
          env = [
            {
              name = "RUST_SRC_PATH";
              value = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
            }
          ];
        };

        formatter = pkgs.alejandra;
      }
    )
    // {
      overlays.default = final: prev: {
        laio = self.packages.default;
      };
    };
}
