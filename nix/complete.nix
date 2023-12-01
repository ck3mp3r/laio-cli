{ laio, shell, pkgs, ... }:

pkgs.stdenv.mkDerivation {
  name = "laio-complete-${shell}";

  buildInputs = [ laio ];
  phases = [ "installPhase" ];

  installPhase = ''
    mkdir -p $out
    laio complete ${shell} > $out/laio-complete
  '';
}
