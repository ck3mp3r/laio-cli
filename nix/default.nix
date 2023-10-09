{ pkgs, targetSystem, ... }:
let

  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
  isCrossCompiling = builtins.currentSystem != targetSystem;

  systemMap = (import ./lib.nix).systemMap;
  current = systemMap builtins.currentSystem;
  target = systemMap targetSystem;

  targetMap = {
    "aarch64-darwin" =
      {
        "target" = "aarch64-apple-darwin";
        "rustPlatform" = pkgs.pkgsCross.aarch64-darwin.rustPlatform;
      };
    "aarch64-linux" =
      {
        "target" = "aarch64-unknown-linux-musl";
        "rustPlatform" = pkgs.pkgsCross.aarch64-multiplatform.rustPlatform;
      };
    "x86_64-darwin" =
      {
        "target" = "x86_64-apple-darwin";
        "rustPlatform" = pkgs.rustPlatform;
      };
    "x86_64-linux" =
      {
        "target" = "x86_64-unknown-linux-musl";
        "rustPlatform" = pkgs.pkgsCross.musl64.rustPlatform;
      };
  };

  rustPlatform =
    if isCrossCompiling then
      targetMap.${targetSystem}.rustPlatform
    else
      pkgs.rustPlatform;

  # if [ "${{ matrix.os }}" == "ubuntu-latest" ]; then
  #   cross build --release --target ${{ matrix.target }} --bin rmux
  # fi
  # if [ "${{ matrix.os }}" == "macos-latest" ]; then
  #   rustup target add ${{ matrix.target }}
  #   cargo build --release --target ${{ matrix.target }} --bin rmux
  # fi

  app = (import ./lib.nix).buildRustPackage { inherit pkgs targetSystem; name = cargoToml.package.name; };

  # Define the Rust application package
  rustApp = rustPlatform.buildRustPackage rec {
    pname = cargoToml.package.name;
    version = cargoToml.package.version;
    src = ./.; # assuming the Nix file is at the root of your project

    # Target for cross-compiling
    # cargoBuildFlags = [ "--target=${targetMap.${targetSystem}.target}" ];

    # Specify the Rust version, e.g., nightly-2022-10-01
    # RUST_TOOLCHAIN_CHANNEL = "nightly-2022-10-01";

    # Ensure we're in release mode for optimization
    release = true;

    # If your project has dependencies, provide them here
    # buildInputs = [ ... ];

    # For more complex setups, you might need postPatch, preBuild, etc.
    # postPatch = ''
    #   ...
    # '';
    cargoLock = {
      lockFile = ./Cargo.lock;
    };
    checkType = "debug";
    meta = with pkgs.lib; {
      description = cargoToml.package.description;
      homepage = cargoToml.package.homepage;
      license = licenses.unlicense;
    };
  };
in
{
  inherit app rustApp;
}