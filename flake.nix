{
  description = "rgpt: A terminal client for ChatGPT";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
    # add compatibility layer to generate shell.nix based on flakes
    flake-compat.url = "https://flakehub.com/f/edolstra/flake-compat/1.tar.gz";
  };
  
  outputs = inputs: with inputs;
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        inherit (pkgs) lib;
        # Will be reused in the dev-shell section
        commonArgs = {
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
          buildInputs = with pkgs; [
            openssl
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            pkgs.libiconv
          ];
        };
        # This is necessary, since we import these libraries directly via git in cargo

        # Define the rfm package
        rfm = pkgs.rustPlatform.buildRustPackage {
          pname = "rgpt";
          version = "0.1.0";
          src = ./.;
          cargoLock = { 
            lockFile = ./Cargo.lock; 
          };

          # use the above build-intputs
          inherit (commonArgs) nativeBuildInputs buildInputs;
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkg-config";

        };
      in rec {
        packages.default = rfm;
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            cargo-edit
            cargo-audit
            cargo-machete
            cargo-bloat
            clippy
            rustfmt
          ] ++ commonArgs.nativeBuildInputs ++ commonArgs.buildInputs;

          # Certain Rust tools won't work without this
          # This can also be fixed by using oxalica/rust-overlay and specifying the rust-src extension
          # See https://discourse.nixos.org/t/rust-src-not-found-and-other-misadventures-of-developing-rust-on-nixos/11570/3?u=samuela. for more details.
          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

          # Required for openssl
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkg-config";
        };
      }
    );
}
