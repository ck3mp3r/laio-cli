{ rmx, shell, pkgs, ... }:

pkgs.stdenv.mkDerivation {
  name = "rmx-complete-${shell}";

  buildInputs = with pkgs; [ rmx ];
  phases = [ "installPhase" ];

  installPhase = ''
    mkdir -p $out
    rmx complete ${shell} > $out/rmx-complete
  '';
}
