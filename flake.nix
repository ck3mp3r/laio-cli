{
  description = "Simple flexbox-inspired layout manager for tmux.";
  inputs = {
    nixpkgs.url = "github:NixOs/nixpkgs";
    flake-parts.url = "github:hercules-ci/flake-parts";
    devshell.url = "github:numtide/devshell";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rustnix = {
      url = "github:ck3mp3r/flakes/fix/rustnix-single-artifact?dir=rustnix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {
    self,
    flake-parts,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["aarch64-darwin" "aarch64-linux" "x86_64-linux"];
      perSystem = {
        config,
        system,
        pkgs,
        ...
      }: let
        overlays = [
          inputs.fenix.overlays.default
          inputs.devshell.overlays.default
        ];
        pkgs = import inputs.nixpkgs {inherit system overlays;};

        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        cargoLock = {lockFile = ./Cargo.lock;};

        # Install data for pre-built releases
        installData = {
          aarch64-darwin = builtins.fromJSON (builtins.readFile ./nix/data/aarch64-darwin.json);
          aarch64-linux = builtins.fromJSON (builtins.readFile ./nix/data/aarch64-linux.json);
          x86_64-linux = builtins.fromJSON (builtins.readFile ./nix/data/x86_64-linux.json);
        };

        # Build regular packages (no archives)
        regularPackages = inputs.rustnix.lib.rust.buildPackage {
          inherit
            cargoToml
            cargoLock
            overlays
            pkgs
            system
            installData
            ;
          fenix = inputs.fenix;
          nixpkgs = inputs.nixpkgs;
          src = ./.;
          packageName = "laio";
          archiveAndHash = false;
        };

        # Build archive packages (creates archive with system name)
        archivePackages = inputs.rustnix.lib.rust.buildPackage {
          inherit
            cargoToml
            cargoLock
            overlays
            pkgs
            system
            installData
            ;
          fenix = inputs.fenix;
          nixpkgs = inputs.nixpkgs;
          src = ./.;
          packageName = "archive";
          archiveAndHash = true;
        };
      in {
        apps = {
          default = {
            type = "app";
            program = "${config.packages.default}/bin/laio";
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

        devShells = {
          default = pkgs.devshell.mkShell {
            packages = [inputs.fenix.packages.${system}.stable.toolchain];
            imports = [
              (pkgs.devshell.importTOML ./devshell.toml)
              "${inputs.devshell}/extra/git/hooks.nix"
            ];
          };
        };

        formatter = pkgs.alejandra;
      };

      flake = {
        overlays.default = final: prev: {
          laio = self.packages.default;
        };
      };
    };
}
