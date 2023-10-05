{
  description = "A simple terminal multiplexer written in Rust";
  inputs = {
    devshell.url = "github:numtide/devshell";
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs/";
  };

  outputs = { self, flake-utils, devshell, nixpkgs, ... }:

    flake-utils.lib.eachDefaultSystem
      (system:
        let

          isCrossCompiling = builtins.currentSystem != system;

          # Function to extract architecture and platform
          extractParts = sys:
            let
              parts = builtins.match "([a-z0-9_]+)-([a-z]+)" sys;
            in
            {
              arch = builtins.elemAt parts 0;
              platform = builtins.elemAt parts 1;
            };

          current = extractParts builtins.currentSystem;
          target = extractParts system;

          toolkit = {
            "aarch64-darwin" =
              {
                "target" = "aarch64-apple-darwin";
                "pkgs" = (import <nixpkgs/lib>).systems.examples.aarch64-darwin;
              };
            "aarch64-linux" =
              {
                "target" = "aarch64-unknown-linux-musl";
                "pkgs" = (import <nixpkgs/lib>).systems.examples.aarch64-multiplatform-musl;
              };
            "x86_64-darwin" =
              {
                "target" = "x86_64-apple-darwin";
                "pkgs" = (import <nixpkgs/lib>).systems.examples.x86_64-darwin;
              };
            "x86_64-linux" =
              {
                "target" = "x86_64-unknown-linux-musl";
                "pkgs" = (import <nixpkgs/lib>).systems.examples.musl64;
              };
          };

          overlays = [ devshell.overlays.default ];
          pkgs = import nixpkgs {
            inherit system overlays;
            crossSystem =
              if isCrossCompiling && target.platform == "linux" then
                toolkit.${system}.pkgs // {
                  rustc.config = toolkit.${system}.target;
                }
              else
                null;
          };

          # if [ "${{ matrix.os }}" == "ubuntu-latest" ]; then
          #   cross build --release --target ${{ matrix.target }} --bin rmux
          # fi
          # if [ "${{ matrix.os }}" == "macos-latest" ]; then
          #   rustup target add ${{ matrix.target }}
          #   cargo build --release --target ${{ matrix.target }} --bin rmux
          # fi

          # rustPlatform =
          #   if isCrossCompiling then
          #     {
          #       "aarch64" = {
          #         "x86_64" = pkgs.pkgsCross.aarch64-multiplatform.rustPlatform;
          #         "aarch64" = pkgs.rustPlatform;
          #       };
          #       "x86_64" = {
          #         "aarch64" = pkgs.pkgsCross.aarch64-multiplatform.rustPlatform;
          #         "x86_64" = pkgs.rustPlatform;
          #       };
          #     }."${target.arch}"."${current.arch}"
          #   else
          #     pkgs.rustPlatform;

          rustPlatform = pkgs.rustPlatform;
          cargoToml = builtins.fromTOML (builtins.readFile (builtins.toString ./. + "/Cargo.toml"));
          rmx = rustPlatform.buildRustPackage {
            pname = cargoToml.package.name;
            version = cargoToml.package.version;

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            checkType = "debug";

            meta = with pkgs.lib; {
              description = cargoToml.package.description;
              homepage = cargoToml.package.homepage;
              license = licenses.unlicense;
            };
          };

          rmx-sha256 = pkgs.runCommand "rmx-sha256" { } ''
            ${pkgs.coreutils}/bin/sha256sum ${rmx}/bin/rmx | ${pkgs.coreutils}/bin/cut -f1 -d' ' > $out
          '';

          individualPackages = with pkgs; {
            inherit
              rmx;
            # tmux;
          };
        in
        {
          packages = individualPackages // {
            inherit rmx-sha256;
            default = pkgs.buildEnv
              {
                name = "rmx";
                paths = builtins.attrValues individualPackages;
              };
          };
          devShells.default = pkgs.devshell.mkShell {
            packages = [ pkgs.cargo pkgs.rustc ];
            imports = [ (pkgs.devshell.importTOML ./devshell.toml) ];
            env = [{
              name = "RUST_SRC_PATH";
              value = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
            }];
          };
        }
      );
}

