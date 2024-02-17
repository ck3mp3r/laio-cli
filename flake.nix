{
  description = "Simple flexbox-inspired layout manager for tmux.";
  inputs = {
    nixpkgs.url = "github:NixOs/nixpkgs/23.05";
    devshell.url = "github:numtide/devshell";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, flake-utils, devshell, nixpkgs, fenix, ... }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          utils = import ./nix/utils.nix;
          overlays = [ devshell.overlays.default ];
          pkgs = import nixpkgs { inherit system overlays; };
          toolchain = with fenix.packages.${system}; combine [
            stable.cargo
            stable.rust-analyzer
            stable.rustc
            targets.aarch64-apple-darwin.stable.rust-std
            targets.aarch64-unknown-linux-musl.stable.rust-std
            targets.x86_64-apple-darwin.stable.rust-std
            targets.x86_64-unknown-linux-musl.stable.rust-std
          ];

          crossPkgs = target:
            let
              isCrossCompiling = target != system;
              config = utils.getTarget target;
              tmpPkgs =
                import
                  nixpkgs
                  {
                    inherit overlays system;
                    crossSystem =
                      if isCrossCompiling || pkgs.stdenv.isLinux then {
                        inherit config;
                        rustc = { inherit config; };
                        isStatic = pkgs.stdenv.isLinux;
                      } else null;
                  };

              toolchain = with fenix.packages.${system}; combine [
                stable.cargo
                stable.rustc
                targets.aarch64-apple-darwin.stable.rust-std
                targets.aarch64-unknown-linux-musl.stable.rust-std
                targets.x86_64-apple-darwin.stable.rust-std
                targets.x86_64-unknown-linux-musl.stable.rust-std
              ];

              callPackage = pkgs.lib.callPackageWith
                (tmpPkgs // { inherit config toolchain; });

            in
            {
              inherit
                callPackage;
              pkgs = tmpPkgs;
            };

        in
        rec {
          apps = {
            default = {
              type = "app";
              program = "${packages.default}/bin/laio";
            };
          };

          packages = {
            default = pkgs.callPackage ./nix/install.nix { };
          } // nixpkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
            laio-x86_64-linux = (crossPkgs "x86_64-linux").callPackage ./nix/build.nix { };
            laio-aarch64-linux = (crossPkgs "aarch64-linux").callPackage ./nix/build.nix { };
          } // nixpkgs.lib.optionalAttrs pkgs.stdenv.isDarwin {
            laio-aarch64-darwin = (crossPkgs "aarch64-darwin").callPackage ./nix/build.nix { };
            laio-x86_64-darwin = (crossPkgs "x86_64-darwin").callPackage ./nix/build.nix { };
          };

          devShells.default = pkgs.devshell.mkShell {
            packages = [ toolchain ];
            imports = [ (pkgs.devshell.importTOML ./devshell.toml) ];
            env = [{
              name = "RUST_SRC_PATH";
              value = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
            }];
          };

          overlays.default = final: prev: {
            laio = self.packages.${system}.default;
          };
        }
      );
}
