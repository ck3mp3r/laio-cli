{ pkgs, system }:
let
  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
  data = builtins.fromJSON (builtins.readFile ./data/${system}.json);
in
pkgs.stdenv.mkDerivation rec {
  pname = cargoToml.package.name;
  version = cargoToml.package.version;

  src = pkgs.fetchurl {
    url = data.url;
    sha256 = data.hash;
  };

  phases = [ "installPhase" ];

  installPhase = ''
    mkdir -p $out/bin
    cp ${src} $out/bin/rmx
    chmod +x $out/bin/rmx
  '';
}
