{
  description = "A simple terminal multiplexer written in Rust";
  inputs = {
    devshell.url = "github:numtide/devshell";
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs/23.05";
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
            stable.rustc
            targets.x86_64-apple-darwin.stable.rust-std
            targets.aarch64-apple-darwin.stable.rust-std
            targets.x86_64-unknown-linux-musl.stable.rust-std
            targets.aarch64-unknown-linux-musl.stable.rust-std
          ];

          crossPkgs = target:
            let
              isCrossCompiling = target != system;
              buildTarget = utils.getTarget target;
              tmpPkgs =
                import
                  nixpkgs
                  {
                    inherit overlays system;
                    crossSystem =
                      if (isCrossCompiling) then {
                        config = buildTarget;
                        rustc.config = buildTarget;
                      } else null;
                  };

              toolchain = with fenix.packages.${system}; combine [
                stable.cargo
                stable.rustc
                targets.x86_64-apple-darwin.stable.rust-std
                targets.aarch64-apple-darwin.stable.rust-std
                targets.x86_64-unknown-linux-musl.stable.rust-std
                targets.aarch64-unknown-linux-musl.stable.rust-std
              ];

              callPackage = nixpkgs.lib.callPackageWith
                (tmpPkgs // { inherit buildTarget toolchain; });

            in
            {
              inherit
                callPackage;
              pkgs = tmpPkgs;
            };

        in
        {
          packages = {
            default = pkgs.callPackage ./nix/install.nix { };
          } // nixpkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
            rmx-x86_64-linux = (crossPkgs "x86_64-linux").callPackage ./nix/build.nix { };
            rmx-aarch64-linux = (crossPkgs "aarch64-linux").callPackage ./nix/build.nix { };
          } // nixpkgs.lib.optionalAttrs pkgs.stdenv.isDarwin {
            rmx-aarch64-darwin = (crossPkgs "aarch64-darwin").callPackage ./nix/build.nix { };
            rmx-x86_64-darwin = (crossPkgs "x86_64-darwin").callPackage ./nix/build.nix { };
          };

          devShells.default = pkgs.devshell.mkShell {
            packages = with pkgs; [ toolchain ];
            imports = [ (pkgs.devshell.importTOML ./devshell.toml) ];
            env = [{
              name = "RUST_SRC_PATH";
              value = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
            }];
          };

          overlay = final: prev: {
            rmx = self.packages.${system}.default;
          };
        }
      );
}
