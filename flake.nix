{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };

    naersk = {
      url = "github:nix-community/naersk/master";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    utils = {
      url = "github:numtide/flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
      inputs.flake-compat.follows = "flake-compat";
    };
  };

  outputs = {
    self,
    nixpkgs,
    utils,
    naersk,
    pre-commit-hooks,
    ...
  }:
    utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {inherit system;};
      naersk-lib = pkgs.callPackage naersk {};
      withOpenSSL = old:
        old
        // {
          OPENSSL_STATIC = old.OPENSSL_STATIC or 0;
          OPENSLL_DIR = old.OPENSSL_DIR or pkgs.openssl.dev;
          buildInputs = (old.buildInputs or []) ++ (with pkgs; [pkg-config openssl]);
        };

      pcc = {withRust}:
        pre-commit-hooks.lib.${system}.run {
          src = ./.;
          hooks =
            {
              alejandra.enable = true;
              deadnix.enable = true;
              statix.enable = true;
            }
            // (pkgs.lib.optionalAttrs withRust {
              rustfmt.enable = true;
              clippy.enable = true;
            });
        };
    in {
      packages = let
        mkNu = {
          doCheck ? false,
          doDoc ? false,
        }:
          naersk-lib.buildPackage (withOpenSSL {
            inherit doCheck doDoc;
            copyLibs = true;
            cargoTestOptions = def: def ++ ["--workspace"];
            src = ./.;
          });
      in rec {
        nu = mkNu {};
        nuWithDocs = mkNu {doDoc = true;};
        nuWithTests = mkNu {
          doDoc = true;
          doCheck = true;
        };
        default = nu;
        checkPretty = pcc {withRust = false;};
      };
      devShells = rec {
        nu = pkgs.mkShell (withOpenSSL {
          inherit (pcc {withRust = true;}) shellHook;
          buildInputs = with pkgs; [cargo rustc rustfmt pre-commit rustPackages.clippy];
          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
        });
        default = nu;
      };

      checks = {
        inherit (self.packages.${system}) nuWithTests checkPretty;
      };
    });
}
