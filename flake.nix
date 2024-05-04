{
  description = "rgpt: A terminal client for ChatGPT";

  inputs = {
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.follows = "rust-overlay/flake-utils";
    nixpkgs.follows = "rust-overlay/nixpkgs";
  };
  
  outputs = inputs: with inputs;
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        code = pkgs.callPackage ./. { inherit pkgs system rust-overlay; };
      in rec {
        packages = {
          rgpt = code.rgpt;
          all = pkgs.symlinkJoin {
            name = "all";
            paths = with code; [ rgpt ];
          };
          default = packages.all;
        };
      }
    );
}
