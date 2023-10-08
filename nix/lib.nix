let
  # Function to extract architecture and platform
  systemMap = sys:
    let
      parts = builtins.match "([a-z0-9_]+)-([a-z]+)" sys;
    in
    {
      arch = builtins.elemAt parts 0;
      platform = builtins.elemAt parts 1;
    };

  buildRustPackage = { pkgs, targetSystem, name, ... }:
    let
      foo = "bar";
    in
    pkgs.stdenv.mkDerivation {
      inherit name;

      buildInputs = with pkgs; [
        rustup
        cargo-cross
      ];

      src = ./.;

      env = {
        "RUSTUP_HOME" = "/tmp";
      };

      buildPhase = ''
        rustup default stable
      '';

      buildCommand = ''
        cargo build
      '';

      installPhase = ''
        # Install the built binary to the `$out` directory
        mkdir -p $out/bin
        cp path-to-your-binary $out/bin/
      '';
    };
in
{
  inherit systemMap buildRustPackage;
}
