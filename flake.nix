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
          pkgs = import nixpkgs { inherit system overlays; };
          cargoToml = builtins.fromTOML (builtins.readFile (builtins.toString ./. + "/Cargo.toml"));
          rmx = pkgs.rustPlatform.buildRustPackage {
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
