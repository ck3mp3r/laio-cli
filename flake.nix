{
  description = "Simple flexbox-inspired layout manager for tmux.";
  inputs = {
    nixpkgs.url = "github:NixOs/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    devenv.url = "github:cachix/devenv";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rustnix = {
      url = "github:ck3mp3r/flakes?dir=rustnix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.fenix.follows = "fenix";
      inputs.devenv.follows = "devenv";
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
        ...
      }: let
        overlays = [
          inputs.fenix.overlays.default
        ];
        pkgs = import inputs.nixpkgs {inherit system overlays;};

        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        cargoLock = {lockFile = ./Cargo.lock;};
        supportedTargets = ["aarch64-darwin" "aarch64-linux" "x86_64-linux"];

        # Install data for pre-built releases
        installData = {
          aarch64-darwin = builtins.fromJSON (builtins.readFile ./data/aarch64-darwin.json);
          aarch64-linux = builtins.fromJSON (builtins.readFile ./data/aarch64-linux.json);
          x86_64-linux = builtins.fromJSON (builtins.readFile ./data/x86_64-linux.json);
        };

        # Build regular packages (no archives)
        regularPackages = inputs.rustnix.lib.rust.buildTargetOutputs {
          inherit
            cargoToml
            cargoLock
            overlays
            pkgs
            system
            installData
            supportedTargets
            ;
          fenix = inputs.fenix;
          nixpkgs = inputs.nixpkgs;
          src = ./.;
          packageName = "laio";
          archiveAndHash = false;
        };

        # Build archive packages (creates archive with system name)
        archivePackages = inputs.rustnix.lib.rust.buildTargetOutputs {
          inherit
            cargoToml
            cargoLock
            overlays
            pkgs
            system
            installData
            supportedTargets
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
          // archivePackages;

        devShells = {
          default = inputs.devenv.lib.mkShell {
            inherit pkgs inputs;
            modules = [
              ./nix/devenv/developer.nix
            ];
          };

          ci = inputs.devenv.lib.mkShell {
            inherit pkgs inputs;
            modules = [
              ./nix/devenv/ci.nix
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
