{ stdenv, installShellFiles, buildTarget, toolchain, pkgs, lib, libiconv }:

let
  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
  shout = string: builtins.replaceStrings [ "-" ] [ "_" ] (pkgs.lib.toUpper string);
in

(pkgs.makeRustPlatform {
  cargo = toolchain;
  rustc = toolchain;
}).buildRustPackage {
  name = cargoToml.package.name;
  version = cargoToml.package.version;

  nativeBuildInputs = with pkgs;[
    installShellFiles
<<<<<<< HEAD
||||||| parent of 40569da (--wip-- [skip ci])
    # stdenv.cc
=======
    autoPatchelfHook
    # stdenv.cc
>>>>>>> 40569da (--wip-- [skip ci])
  ] ++ lib.optionals stdenv.isLinux [
    patchelf
  ] ++ lib.optionals stdenv.isDarwin [
  ];

  buildInputs = with pkgs; [
  ] ++ lib.optionals stdenv.isDarwin [
    # libiconv
  ];

  src = ../.;

  cargoLock.lockFile = ../Cargo.lock;

<<<<<<< HEAD
||||||| parent of 40569da (--wip-- [skip ci])
  CARGO_BUILD_TARGET = buildTarget;
  RUSTFLAGS =
    if stdenv.isLinux then
      [
        "-C link-arg=-static"
      ]
    else
      [
        # "-C link-arg=-static"
        # "-C target-feature=+crt-static"
      ];

  # CARGO_BUILD_RUSTFLAGS =
  #   if stdenv.isLinux then
  #     [ "-C linker=rust-lld" ]
  #   else
  #     [
  #       # "-C link-arg=-static"
  #       # "-C target-feature=+crt-static"
  #     ];
  # "CARGO_TARGET_${shout buildTarget}_LINKER" = "${pkgs.stdenv.cc.targetPrefix}ld";
  # NIX_LDFLAGS = lib.optionalString stdenv.isDarwin "-framework System";

  # CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER =
  #   let
  #     inherit (pkgs.pkgsCross.aarch64-multiplatform.stdenv) cc;
  #   in
  #   "${cc}/bin/${cc.targetPrefix}cc";

  # CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER =
  #   let
  #     inherit (pkgs.pkgsCross.musl64.stdenv) cc;
  #   in
  #   "${cc}/bin/${cc.targetPrefix}cc";

  # CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER =
  #   let
  #     inherit (pkgs.pkgsCross.aarch64-darwin.stdenv) cc;
  #   in
  #   "${cc}/bin/${cc.targetPrefix}cc";

  # CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER =
  #   let
  #     inherit (pkgs.stdenv) cc;
  #   in
  #   "${cc}/bin/${cc.targetPrefix}cc";

=======
  CARGO_BUILD_TARGET = buildTarget;
  RUSTFLAGS =
    if stdenv.isLinux then
      [
  #      "-C linker=rust-lld"
      ]
    else
      [
        # "-C link-arg=-static"
        # "-C target-feature=+crt-static"
      ];

  # CARGO_BUILD_RUSTFLAGS =
  #   if stdenv.isLinux then
  #     [ "-C linker=rust-lld" ]
  #   else
  #     [
  #       # "-C link-arg=-static"
  #       # "-C target-feature=+crt-static"
  #     ];
  # "CARGO_TARGET_${shout buildTarget}_LINKER" = "${pkgs.stdenv.cc.targetPrefix}ld";
  # NIX_LDFLAGS = lib.optionalString stdenv.isDarwin "-framework System";

  # CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER =
  #   let
  #     inherit (pkgs.pkgsCross.aarch64-multiplatform.stdenv) cc;
  #   in
  #   "${cc}/bin/${cc.targetPrefix}cc";

  # CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER =
  #   let
  #     inherit (pkgs.pkgsCross.musl64.stdenv) cc;
  #   in
  #   "${cc}/bin/${cc.targetPrefix}cc";

  # CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER =
  #   let
  #     inherit (pkgs.pkgsCross.aarch64-darwin.stdenv) cc;
  #   in
  #   "${cc}/bin/${cc.targetPrefix}cc";

  # CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER =
  #   let
  #     inherit (pkgs.stdenv) cc;
  #   in
  #   "${cc}/bin/${cc.targetPrefix}cc";

  checkPhase = ''
  '';

  installPhase = ''
    runHook preInstall
    install -m755 -D ${buildTarget}/rmx $out/bin/rmx
    runHook postInstall
  '';

>>>>>>> 40569da (--wip-- [skip ci])
  meta = with pkgs.lib;
    {
      description = cargoToml.package.description;
      homepage = cargoToml.package.homepage;
      license = licenses.unlicense;
    };
}
