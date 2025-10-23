{ pkgs, inputs, ... }:

{
  # Directly add fenix toolchain - no language modules
  packages = [
    inputs.fenix.packages.${pkgs.system}.stable.toolchain
    pkgs.cargo-tarpaulin
    pkgs.zola
    pkgs.act
  ];

  scripts = {
    checks.exec = "nix flake check";
    tests.exec = "cargo test";
    clippy.exec = "cargo clippy $@";
    clean.exec = "cargo clean";
    coverage.exec = "cargo tarpaulin --out Html";
  };

  git-hooks.hooks.pre-push = {
    enable = true;
    entry = "cargo test -- --include-ignored";
    stages = [ "pre-push" ];
  };

  enterShell = ''
    echo "laio devshell"
  '';
}