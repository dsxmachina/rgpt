{ pkgs, system, rust-overlay }:
{
  rgpt = pkgs.rustPlatform.buildRustPackage {
    pname = "rgpt";
    version = "0.1.0";
    src = ./.;
  
    cargoLock = { lockFile = ./Cargo.lock; };
  
    nativeBuildInputs = with pkgs; [ pkg-config ];
    buildInputs = with pkgs; [ openssl ];
    PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkg-config";
  };
}
