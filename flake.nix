{
  description = "Simple flexbox-inspired layout manager for tmux.";
  inputs = {
    base-nixpkgs.url = "github:ck3mp3r/flakes?dir=base-nixpkgs";
    nixpkgs.follows = "base-nixpkgs/unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    rustnix = {
      url = "github:ck3mp3r/flakes?dir=rustnix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.base-nixpkgs.follows = "base-nixpkgs";
      inputs.flake-parts.follows = "flake-parts";
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
        pkgs = import inputs.nixpkgs {inherit system;};

        cargoToml = fromTOML (builtins.readFile ./Cargo.toml);
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
            pkgs
            system
            installData
            supportedTargets
            ;
          nixpkgs = inputs.nixpkgs;
          overlays = [];
          src = ./.;
          packageName = "laio";
          archiveAndHash = false;
        };

        # Build archive packages (creates archive with system name)
        archivePackages = inputs.rustnix.lib.rust.buildTargetOutputs {
          inherit
            cargoToml
            cargoLock
            pkgs
            system
            installData
            supportedTargets
            ;
          nixpkgs = inputs.nixpkgs;
          overlays = [];
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
          default = pkgs.mkShell {
            packages = with pkgs; [
              (inputs.rustnix.lib.rust.mkToolchain {
                inherit system;
                extras = ["clippy" "rustfmt"];
              })
              cargo-tarpaulin
              zola
              act
            ];

            shellHook = ''
              echo "laio devshell"
            '';
          };

          ci = pkgs.mkShell {
            packages = [
              (inputs.rustnix.lib.rust.mkToolchain {
                inherit system;
                extras = ["clippy"];
              })
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
