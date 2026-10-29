{
  description = "Simple flexbox-inspired layout manager for tmux.";
  inputs = {
    base-nixpkgs = {
      url = "github:ck3mp3r/flakes?dir=base-nixpkgs";
      inputs.flake-parts.follows = "flake-parts";
    };
    nixpkgs.follows = "base-nixpkgs/unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    rustnix = {
      url = "github:ck3mp3r/flakes?dir=rustnix";
      inputs = {
        base-nixpkgs.follows = "base-nixpkgs";
        nixpkgs.follows = "nixpkgs";
        flake-parts.follows = "flake-parts";
      };
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

        # LLVM tools for cargo-llvm-cov; must match the LLVM version used by rustc
        llvmPkgs = pkgs.llvmPackages_23;

        # cargo-llvm-cov tests break on nixpkgs rust bumps; binary still works (nixpkgs#197478)
        cargo-llvm-cov = pkgs.cargo-llvm-cov.overrideAttrs (_: {doCheck = false;});

        # Coverage report wrapper (HTML output in coverage/)
        coverage = pkgs.writeShellScriptBin "coverage" ''cargo llvm-cov nextest --workspace --cargo-profile coverage --html --output-dir coverage/ --ignore-filename-regex '(/test/|/test$)${"'"}'';

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
                extras = ["clippy" "rustfmt" "llvm-tools-preview"];
              })
              cargo-nextest
              cargo-llvm-cov
              coverage
              zola
              act
              prek
            ];

            # cargo-llvm-cov locates the LLVM binaries through these variables
            LLVM_COV = "${llvmPkgs.llvm}/bin/llvm-cov";
            LLVM_PROFDATA = "${llvmPkgs.llvm}/bin/llvm-profdata";

            shellHook = ''
              echo "laio devshell"
              echo "coverage - cargo llvm-cov (HTML report in coverage/)"
            '';
          };

          ci = pkgs.mkShell {
            packages = [
              (inputs.rustnix.lib.rust.mkToolchain {
                inherit system;
                extras = ["clippy" "rustfmt"];
              })
              pkgs.cargo-nextest
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
