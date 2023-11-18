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

  meta = with pkgs.lib;
    {
      description = cargoToml.package.description;
      homepage = cargoToml.package.homepage;
      license = licenses.unlicense;
    };
}
