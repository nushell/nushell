{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self, ... }@inputs:
    let
      inherit (inputs.nixpkgs) lib;
      # systems = lib.systems.flakeExposed;
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "powerpc64le-linux"
        "aarch64-darwin"
        "riscv64-linux"
        "armv6l-linux"
        "armv7l-linux"
        "i686-linux"
        ## Broken in `nix flake check`
        # "x86_64-freebsd"
      ];
      main-overlay = (
        final: previous: {
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
          rustToolchain-latest-devshell = previous.rust-bin.stable.latest.default.override ({
            extensions = [
              "rust-src"
              "rust-analyzer"
            ];
          });
          rustPlatform-latest-devshell = final.makeRustPlatform {
            cargo = final.rustToolchain-latest-devshell;
            rustc = final.rustToolchain-latest-devshell;
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
                  buildInputs = [
                    previous.openssl
                    previous.libgit2
                  ];
                };
                inc = { };
                polars = {
                  buildInputs = [
                    previous.openssl
                    previous.aws-lc
                  ]
                  ++ final.nushell.buildInputs;
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
        }
      );
      all-overlays = lib.composeManyExtensions [
        inputs.rust-overlay.overlays.default
        main-overlay
      ];
      forEachSupportedSystem =
        f:
        inputs.nixpkgs.lib.genAttrs systems (
          system:
          f {
            pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [
                all-overlays
              ];
            };
          }
        );
    in
    {
      overlays = {
        default = main-overlay;
        all = all-overlays;
      };
      packages = forEachSupportedSystem (
        { pkgs, ... }:
        {
          inherit (pkgs) nushell;
          dev = pkgs.nushell.override {
            buildType = "debug";
          };
          default = pkgs.nushell;
          all-dev = pkgs.symlinkJoin {
            inherit (pkgs.nushell) version meta;
            pname = "nushellAll";
            paths = [
              self.packages.${pkgs.stdenv.hostPlatform.system}.dev
            ]
            ++ (builtins.attrValues self.legacyPackages.${pkgs.stdenv.hostPlatform.system}.nushellPluginsDev);
          };
          all = pkgs.symlinkJoin {
            inherit (pkgs.nushell) version meta;
            pname = "nushellAll";
            paths = [
              pkgs.nushell
            ]
            ++ (builtins.attrValues self.legacyPackages.${pkgs.stdenv.hostPlatform.system}.nushellPlugins);
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
          nushellPluginsDev = (
            lib.mapAttrs (
              _: plugin: plugin.override { buildType = "debug"; }
            ) self.legacyPackages.${pkgs.stdenv.hostPlatform.system}.nushellPlugins
          );
        }
      );
      devShells = forEachSupportedSystem (
        { pkgs, ... }:
        let
          basepkgs =
            with pkgs;
            (
              [
                curlMinimal
                openssl
                pkg-config
                zstd
                zlib
                libgit2
              ]
              ++ (lib.optionals stdenv.hostPlatform.isLinux [
                python3
                libX11
              ])
              ++ (lib.optionals stdenv.hostPlatform.isDarwin [
                nghttp2
              ])
            );
        in
        {
          # devshell with build/check deps from nushell and plugins, with some
          # separation for linux/darwin.
          default = pkgs.mkShell {
            packages =
              with pkgs;
              [
                rustToolchain-devshell
                rustPlatform-devshell.bindgenHook
              ]
              ++ basepkgs;
          };
          latest = pkgs.mkShell {
            packages =
              with pkgs;
              [
                rustToolchain-latest-devshell
                rustPlatform-latest-devshell.bindgenHook
              ]
              ++ basepkgs;
          };
        }
      );
    };
}
