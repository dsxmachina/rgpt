{ pkgs ? import <nixpkgs> {}}:
with pkgs;
mkShell {
  buildInputs = [ 
    openssl 
    pkg-config
  ];
  PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkg-config";
}
