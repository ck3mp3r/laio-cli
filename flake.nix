{
  description = "Simple flexbox-inspired layout manager for tmux.";
  inputs = {
    nixpkgs.url = "github:NixOs/nixpkgs";
    devshell.url = "github:numtide/devshell";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    flake-utils,
    devshell,
    nixpkgs,
    fenix,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [devshell.overlays.default];
        pkgs = import nixpkgs {inherit system overlays;};
        toolchain = with fenix.packages.${system};
          combine [
            stable.cargo
            stable.rust-analyzer
            stable.rustc
            stable.rustfmt
            stable.clippy
            targets.aarch64-apple-darwin.stable.rust-std
            targets.aarch64-unknown-linux-musl.stable.rust-std
            targets.x86_64-apple-darwin.stable.rust-std
            targets.x86_64-unknown-linux-musl.stable.rust-std
          ];

        laioPackages = import ./nix/packages.nix {
          inherit
            fenix
            nixpkgs
            overlays
            pkgs
            system
            ;
        };
      in rec {
        apps = {
          default = {
            type = "app";
            program = "${packages.default}/bin/laio";
          };
        };

        packages =
          laioPackages
          // {
            tmux-mcp-tools = let
              cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
            in
              pkgs.stdenvNoCC.mkDerivation {
                pname = "mcp-tools";
                version = cargoToml.package.version;
                src = ./mcp-tools;

                installPhase = ''
                  mkdir -p $out/share/nushell/mcp-tools/tmux
                  cp tmux.nu $out/share/nushell/mcp-tools/tmux/tmux.nu
                '';
              };
          };

        devShells.default = pkgs.devshell.mkShell {
          packages = [toolchain];
          imports = [(pkgs.devshell.importTOML ./devshell.toml) "${devshell}/extra/git/hooks.nix"];
          env = [
            {
              name = "RUST_SRC_PATH";
              value = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
            }
          ];
        };

        formatter = pkgs.alejandra;
      }
    )
    // {
      overlays.default = final: prev: {
        laio = self.packages.default;
      };
    };
}
