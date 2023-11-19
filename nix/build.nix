{ stdenv, installShellFiles, config, toolchain, pkgs, lib, libiconv }:

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
    # autoPatchelfHook
    # patchelf
    # python3
    # installShellFiles
    # autoPatchelfHook
    # python3
  ] ++ lib.optionals stdenv.isLinux [
    # patchelf
  ] ++ lib.optionals stdenv.isDarwin [
  ];

  buildInputs = with pkgs; [
  ] ++ lib.optionals stdenv.isLinux [
    # stdenv.cc.cc.lib
  ] ++ lib.optionals stdenv.isDarwin [
    # libiconv
  ];

  src = ../.;

  RUSTFLAGS = [

  ] ++ pkgs.lib.optionals stdenv.isLinux [
    # "-C target-feature=+crt-static"
  ];

  cargoLock.lockFile = ../Cargo.lock;

  installPhase = ''
    # runHook preInstall
    install -m755 -D target/${config}/release/rmx $out/bin/rmx
    # runHook postInstall
  '';

  meta = with pkgs.lib;
    {
      description = cargoToml.package.description;
      homepage = cargoToml.package.homepage;
      license = licenses.unlicense;
    };
}
