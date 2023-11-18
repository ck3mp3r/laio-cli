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
    tree 
    installShellFiles
  ] ++ lib.optionals stdenv.isLinux [
    autoPatchelfHook
    patchelf
  ] ++ lib.optionals stdenv.isDarwin [
  ];

  buildInputs = with pkgs; [
  ] ++ lib.optionals stdenv.isDarwin [
    # libiconv
  ];

  src = ../.;

  cargoLock.lockFile = ../Cargo.lock;

  installPhase = ''
    runHook preInstall
    tree target/
    install -m755 -D target/${buildTarget}/release/rmx $out/bin/rmx
    runHook postInstall
  '';

  meta = with pkgs.lib;
    {
      description = cargoToml.package.description;
      homepage = cargoToml.package.homepage;
      license = licenses.unlicense;
    };
}
