{}:
let
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
in
rmx
