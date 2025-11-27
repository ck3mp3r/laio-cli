{
  pkgs,
  inputs,
  ...
}: {
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

  git-hooks.hooks = {
    fix-whitespace = {
      enable = true;
      name = "Fix trailing whitespace";
      entry = "${pkgs.writeShellScript "fix-whitespace" ''
        # Fix trailing whitespace in staged files
        git diff --cached --name-only --diff-filter=ACM | while read file; do
          if [ -f "$file" ]; then
            ${pkgs.gnused}/bin/sed -i 's/[[:space:]]*$//' "$file"
            git add "$file"
          fi
        done
      ''}";
      language = "system";
      stages = ["pre-commit"];
      pass_filenames = false;
    };

    pre-push = {
      enable = true;
      entry = "cargo test -- --include-ignored";
      stages = ["pre-push"];
    };
  };

  enterShell = ''
    echo "laio devshell"
  '';
}
