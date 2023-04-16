{
  description = "A simple terminal multiplexer written in Rust";
  inputs = {
    devshell.url = "github:numtide/devshell";
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, flake-utils, devshell, nixpkgs, ... }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ devshell.overlays.default ];
          };
          cargoToml = builtins.fromTOML (builtins.readFile (builtins.toString ./. + "/Cargo.toml"));
          rmux = pkgs.rustPlatform.buildRustPackage
            {
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
        in
        {
          defaultPackage = rmux;
          devShell =
            pkgs.devshell.mkShell {
              packages = [ pkgs.cargo pkgs.rustc rmux ];
              imports = [ (pkgs.devshell.importTOML ./devshell.toml) ];
            };
        }
      );
}
