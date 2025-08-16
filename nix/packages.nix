{
  fenix,
  nixpkgs,
  overlays,
  pkgs,
  system,
  ...
}: let
  utils = import ./utils.nix;
  crossPkgs = target: let
    isCrossCompiling = target != system;
    config = utils.getTarget target;
    tmpPkgs = import nixpkgs {
      inherit overlays system;
      crossSystem =
        if isCrossCompiling || pkgs.stdenvNoCC.isLinux
        then {
          inherit config;
          rustc = {inherit config;};
          isStatic = pkgs.stdenvNoCC.isLinux;
        }
        else null;
    };

    toolchain = with fenix.packages.${system};
      combine [
        stable.cargo
        stable.rustc
        targets.aarch64-apple-darwin.stable.rust-std
        targets.aarch64-unknown-linux-musl.stable.rust-std
        targets.x86_64-apple-darwin.stable.rust-std
        targets.x86_64-unknown-linux-musl.stable.rust-std
      ];

    callPackage = pkgs.lib.callPackageWith (tmpPkgs // {inherit config toolchain;});
  in {
    inherit
      callPackage
      ;
    pkgs = tmpPkgs;
  };
in
  {
    default = pkgs.callPackage ./install.nix {};
  }
  // pkgs.lib.optionalAttrs pkgs.stdenvNoCC.isLinux {
    laio-x86_64-linux = (crossPkgs "x86_64-linux").callPackage ./build.nix {};
    laio-aarch64-linux = (crossPkgs "aarch64-linux").callPackage ./build.nix {};
  }
  // pkgs.lib.optionalAttrs pkgs.stdenvNoCC.isDarwin {
    laio-aarch64-darwin = (crossPkgs "aarch64-darwin").callPackage ./build.nix {};
    laio-x86_64-darwin = (crossPkgs "x86_64-darwin").callPackage ./build.nix {};
  }
