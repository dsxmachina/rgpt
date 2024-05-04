{
  description = "rgpt: A terminal client for ChatGPT";

  inputs = {
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.follows = "rust-overlay/flake-utils";
    nixpkgs.follows = "rust-overlay/nixpkgs";
  };

  outputs = { self, nixpkgs }: {
    packages.x86_64-linux.rgpt =  nixpkgs.callPackage ./. { inherit nixpkgs; };
    packages.x86_64-linux.default = self.packages.x86_64-linux.rgpt;
  };
}
