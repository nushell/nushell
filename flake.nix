{
  description = "A new type of shell";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs-mozilla = {
      url = "github:mozilla/nixpkgs-mozilla";
      flake = false;
    };
  };

  outputs = { self, flake-utils, naersk, nixpkgs, nixpkgs-mozilla }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import nixpkgs) {
          inherit system;

          overlays = [ (import nixpkgs-mozilla) ];
        };

        toolchain = (pkgs.rustChannelOf {
          rustToolchain = ./rust-toolchain.toml;
          sha256 = "sha256-R0F0Risbr74xg9mEYydyebx/z0Wu6HI0/KWwrV30vZo=";
        }).rust;

        naersk' = pkgs.callPackage naersk {
          cargo = toolchain;
          rustc = toolchain;
        };

      in rec {
        # For `nix build` & `nix run`:
        defaultPackage = naersk'.buildPackage {
          src = ./.;
          buildInputs = [ pkgs.pkg-config pkgs.openssl ];
        };

        # For `nix develop` (optional, can be skipped):
        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ rustc cargo ];
        };
      }
    );
}

