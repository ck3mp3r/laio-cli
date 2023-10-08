{
  description = "A simple terminal multiplexer written in Rust";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    devshell.url = "github:numtide/devshell";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, flake-utils, devshell, nixpkgs, fenix, naersk, ... }:

    flake-utils.lib.eachDefaultSystem
      (system:
        let

          isCrossCompiling = builtins.currentSystem != system;
          overlays = [ devshell.overlays.default fenix.overlays.default ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };

          cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
          rmx =
            let
              pkgs = nixpkgs.legacyPackages.${system};
              target = "aarch64-unknown-linux-gnu";
              toolchain = with fenix.packages.${system}; combine [
                minimal.cargo
                minimal.rustc
                targets.${target}.latest.rust-std
              ];
            in

            (naersk.lib.${system}.override {
              cargo = toolchain;
              rustc = toolchain;
            }).buildPackage {
              src = ./.;
              CARGO_BUILD_TARGET = target;
              CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER =
                let
                  inherit (pkgs.pkgsCross.aarch64-multiplatform.stdenv) cc;
                in
                "${cc}/bin/${cc.targetPrefix}cc";
            };
          # let
          #   toolchain = fenix.packages.${builtins.currentSystem}.stable.toolchain;
          # in

          # (pkgs.makeRustPlatform {
          #   cargo = toolchain;
          #   rustc = toolchain;
          # }).buildRustPackage {
          #   pname = cargoToml.package.name;
          #   version = cargoToml.package.version;

          #   src = ./.;

          #   cargoLock.lockFile = ./Cargo.lock;
          # };
          # rmx-sha256 = pkgs.runCommand "rmx-sha256" { } ''
          #   ${pkgs.coreutils}/bin/sha256sum ${rmx}/bin/rmx | ${pkgs.coreutils}/bin/cut -f1 -d' ' > $out
          # '';

          individualPackages = with pkgs; {
            inherit
              rmx;
            # tmux;
          };
        in
        {
          packages = individualPackages // {
            # inherit rmx-sha256;
            default = pkgs.buildEnv
              {
                name = "rmx";
                paths = builtins.attrValues individualPackages;
              };
          };
          devShells.default = pkgs.devshell.mkShell {
            packages = with pkgs; [

            ];
            imports = [ (pkgs.devshell.importTOML ./devshell.toml) ];
            env = [{
              name = "RUST_SRC_PATH";
              value = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
            }];
          };
        }
      );
}

