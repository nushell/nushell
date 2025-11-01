{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs:
    let
      inherit (inputs.nixpkgs) lib;
      # systems = lib.systems.flakeExposed;
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "powerpc64le-linux"
        "x86_64-darwin"
        "aarch64-darwin"
        "riscv64-linux"
        "armv6l-linux"
        "armv7l-linux"
        "i686-linux"
        ## Broken in `nix flake check`
        # "x86_64-freebsd"
      ];
      main-overlay = final: previous: {
        # Override the rustPlatform without forcing a rebuild of _every_ tool as well
        rustToolchain' = previous.rust-bin.fromRustupToolchainFile ../../rust-toolchain.toml;
        rustPlatform' = final.makeRustPlatform {
          cargo = final.rustToolchain';
          rustc = final.rustToolchain';
        };
        # Add a specific rustPlatform with extensions for development/editors
        rustToolchain-devshell = final.rustToolchain'.override ({
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        });
        rustPlatform-devshell = final.makeRustPlatform {
          cargo = final.rustToolchain-devshell;
          rustc = final.rustToolchain-devshell;
        };
        nushell = previous.callPackage ./. {
          rustPlatform = final.rustPlatform';
        };
        nushellPlugins =
          previous.nushellPlugins
          // (builtins.mapAttrs
            (
              pname: d:
              previous.callPackage ./. (
                {
                  pname = "nu_plugin_${pname}";
                  rustPlatform = final.rustPlatform';
                  extraCargo = ../../crates + "/nu_plugin_${pname}/Cargo.toml";
                  buildAndTestSubdir = "crates/nu_plugin_${pname}";
                  nativeBuildInputs = [
                    previous.pkg-config
                  ]
                  ++ lib.optionals previous.stdenv.cc.isClang [ final.rustPlatform'.bindgenHook ]
                  ++ (d.extraNativeBuildInputs or [ ]);
                }
                // d
              )
            )
            {
              formats = { };
              gstat = {
                buildInputs = [ previous.openssl ];
              };
              inc = { };
              polars = {
                buildInputs = [ previous.openssl ];
                checkFlags = [
                  "--skip=dataframe::command::core::to_repr::test::test_examples"
                ];
              };
              query = {
                buildInputs = [
                  previous.openssl
                  previous.curlMinimal
                ];
              };
            }
          );
      };
      forEachSupportedSystem =
        f:
        inputs.nixpkgs.lib.genAttrs systems (
          system:
          f {
            pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [
                inputs.rust-overlay.overlays.default
                main-overlay
              ];
            };
          }
        );
    in
    {
      overlays = {
        default = main-overlay;
      };
      packages = forEachSupportedSystem (
        { pkgs, ... }:
        {
          inherit (pkgs) nushell;
          default = pkgs.nushell;
          dev = pkgs.nushell.override {
            buildType = "dev";
          };
        }
      );
      legacyPackages = forEachSupportedSystem (
        { pkgs, ... }:
        {
          nushellPlugins = {
            # Only getting a subset of these so we don't have _all_ of the
            # plugins that are in `nixpkgs`
            inherit (pkgs.nushellPlugins)
              formats
              gstat
              inc
              polars
              query
              ;
          };
        }
      );
      devShells = forEachSupportedSystem (
        { pkgs, ... }:
        {
          # devshell with build/check deps from nushell and plugins, with some
          # separation for linux/darwin.
          default = pkgs.mkShell {
            packages =
              with pkgs;
              (
                [
                  rustToolchain-devshell
                  rustPlatform-devshell.bindgenHook
                  curlMinimal
                  openssl
                  pkg-config
                  zstd
                ]
                ++ (lib.optionals stdenv.hostPlatform.isLinux [
                  python3
                  xorg.libX11
                ])
                ++ (lib.optionals stdenv.hostPlatform.isDarwin [
                  zlib
                  nghttp2
                  libgit2
                ])
              );
          };
        }
      );
    };
}
