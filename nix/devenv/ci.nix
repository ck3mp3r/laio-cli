{ pkgs, inputs, ... }:

{
  # Minimal CI environment - only essentials for build processes
  packages = [
    inputs.fenix.packages.${pkgs.system}.stable.toolchain
  ];

  # No scripts, git hooks, or development tools needed for CI
}