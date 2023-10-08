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
          targetMap = {
            "aarch64-darwin" =
              {
                "target" = "aarch64-apple-darwin";
                "crossSystem" = (import <nixpkgs/lib>).systems.examples.aarch64-darwin // {
                  rustc.config = "aarch64-apple-darwin";
                };
              };
            "aarch64-linux" =
              {
                "target" = "aarch64-unknown-linux-musl";
                "crossSystem" = (import <nixpkgs/lib>).systems.examples.aarch64-multiplatform-musl // {
                  rustc.config = "aarch64-unknown-linux-musl";
                };
              };
            "x86_64-darwin" =
              {
                "target" = "x86_64-apple-darwin";
                "crossSystem" = (import <nixpkgs/lib>).systems.examples.x86_64-darwin // {
                  rustc.config = "x86_64-apple-darwin";
                };
              };
            "x86_64-linux" =
              {
                "target" = "x86_64-unknown-linux-musl";
                "crossSystem" = (import <nixpkgs/lib>).systems.examples.musl64 // {
                  rustc.config = "x86_64-unknown-linux-musl";
                };
              };
          };

          pkgs = import nixpkgs {
            inherit overlays;
            system = builtins.currentSystem;
            crossSystem =
              if isCrossCompiling then
                targetMap.${system}.crossSystem
              else
                null;
          };

          cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);

          rmx = pkgs.rustPlatform.buildRustPackage rec {
            pname = cargoToml.package.name;
            version = cargoToml.package.version;

            src = ./.;

            checkType = "debug";

            RUST_BACKTRACE = 1;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

          };

          individualPackages = with pkgs;
            {
              inherit
                rmx;
              # tmux;
            };
        in
        {
          packages = individualPackages // {
            default = pkgs.buildEnv
              {
                name = "rmx";
                paths = builtins.attrValues individualPackages;
              };
          };
          devShells.default = pkgs.devshell.mkShell {
            packages = with pkgs; [ ];
            imports = [ (pkgs.devshell.importTOML ./devshell.toml) ];
            env = [{
              name = "RUST_SRC_PATH";
              value = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
            }];
          };
        }
      );
}

