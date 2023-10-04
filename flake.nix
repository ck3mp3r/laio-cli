{
  description = "A simple terminal multiplexer written in Rust";
  inputs = {
    devshell.url = "github:numtide/devshell";
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs";
  };

  outputs = { self, flake-utils, devshell, nixpkgs, ... }:

    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ devshell.overlays.default ];

          pkgs = import nixpkgs {
            inherit system overlays;
            crossSystem =
              if system == "aarch64-linux" then
                {
                  config = "aarch64-unknown-linux-musl";
                  rustc.config = "aarch64-unknown-linux-musl";
                  isStatic = true;
                }
              else null;
            config = {
              binfmt.emulatedSystems = [ "aarch64-linux" ];
            };
          };

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

          currentParts = extractParts builtins.currentSystem;
          targetParts = extractParts system;

          rustPlatform =
            if isCrossCompiling then
              if targetParts.arch == "aarch64" && currentParts.arch == "x86_64" then
                pkgs.pkgsCross.aarch64-multiplatform.rustPlatform
              else if targetParts.arch == "x86_64" && currentParts.arch == "aarch64" then
                pkgs.pkgsCross.x86_64-multiplatform.rustPlatform
              else
                pkgs.rustPlatform
            else
              pkgs.rustPlatform;

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
              rmx
              tmux;
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
