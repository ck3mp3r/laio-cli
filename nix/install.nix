{
  pkgs,
  system,
}: let
  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
  data = builtins.fromJSON (builtins.readFile ./data/${system}.json);
in
  pkgs.stdenvNoCC.mkDerivation rec {
    pname = cargoToml.package.name;
    version = cargoToml.package.version;

    src = pkgs.fetchurl {
      url = data.url;
      sha256 = data.hash;
    };

    phases = ["unpackPhase" "installPhase"];

    unpackPhase = ''
      tar -xzf ${src}
    '';

    installPhase = ''
      mkdir -p $out/bin
      cp laio $out/bin/laio
      chmod +x $out/bin/laio
    '';
  }
