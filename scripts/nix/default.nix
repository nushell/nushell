{
  stdenv,
  lib,
  rustPlatform,
  # Allows overriding for the plugins
  pname ? "nushell",
  # Use this for the cargo file that the plugin has
  extraCargo ? ../../Cargo.toml,
  withDefaultFeatures ? true,
  additionalFeatures ? (p: p),
  # Package dependencies
  pkg-config,
  openssl,
  curlMinimal,
  python3,
  xorg,
  nghttp2,
  libgit2,
  zstd,
  zlib,
  nativeBuildInputs ? [
    pkg-config
  ]
  ++ lib.optionals (withDefaultFeatures && stdenv.hostPlatform.isLinux) [ python3 ]
  ++ lib.optionals stdenv.cc.isClang [ rustPlatform.bindgenHook ],
  buildInputs ? [
    zstd
  ]
  ++ lib.optionals stdenv.hostPlatform.isDarwin [ zlib ]
  ++ lib.optionals (withDefaultFeatures && stdenv.hostPlatform.isLinux) [ xorg.libX11 ]
  ++ lib.optionals (withDefaultFeatures && stdenv.hostPlatform.isDarwin) [
    nghttp2
    libgit2
  ],
  checkInputs ?
    lib.optionals stdenv.hostPlatform.isDarwin [ curlMinimal ]
    ++ lib.optionals stdenv.hostPlatform.isLinux [ openssl ],

  # The skipped tests all fail in the sandbox because in the nushell test playground,
  # the tmp $HOME is not set, so nu falls back to looking up the passwd dir of the build
  # user (/var/empty). The assertions however do respect the set $HOME.
  testsToSkip ? [
    "repl::test_config_path::test_default_config_path"
    "repl::test_config_path::test_xdg_config_bad"
    "repl::test_config_path::test_xdg_config_empty"
  ],
  buildAndTestSubdir ? ".",
}:
let
  inherit (builtins)
    fromTOML
    readFile
    head
    ;
  inherit (lib.fileset)
    toSource
    intersection
    unions
    difference
    ;
  root = ../..;
  cargoToml = fromTOML (readFile (root + /Cargo.toml));
  # Allow for extra Cargo.toml files to be passed for crates
  extraToml = fromTOML (readFile extraCargo);
in
rustPlatform.buildRustPackage {
  inherit
    pname
    nativeBuildInputs
    buildInputs
    checkInputs
    buildAndTestSubdir
    ;
  inherit (cargoToml.package) version;

  # Crates will need to build with their Cargo.toml
  src = toSource {
    inherit root;
    fileset = (
      intersection root (
        difference (unions (
          map (p: root + p) [
            /Cargo.toml
            /Cargo.lock
            /src
            /crates
            /tests
            /assets
            /benches
            /toolkit
            /scripts
          ]
        )) ./.
      )
    );
  };

  cargoLock = {
    lockFile = root + /Cargo.lock;
    # Required for some of the dependencies in this repo
    allowBuiltinFetchGit = true;
  };

  buildNoDefaultFeatures = !withDefaultFeatures;
  buildFeatures = additionalFeatures [ ];

  preCheck = ''
    export NU_TEST_LOCALE_OVERRIDE="en_US.UTF-8"
  '';
  checkPhase = ''
    runHook preCheck
    (
      set -x
      pushd ${buildAndTestSubdir}
      HOME=$(mktemp -d) cargo test -j $NIX_BUILD_CORES --offline -- \
        --test-threads=$NIX_BUILD_CORES ${toString (map (p: "--skip=${p}") testsToSkip)}
    )
    runHook postCheck
  '';

  meta = {
    description = extraToml.package.description or cargoToml.package.description;
    homepage =
      if cargoToml == extraToml then cargoToml.package.homepage else extraToml.package.repository or null;
    license = lib.licenses.mit;
    mainProgram = (head (extraToml.bin or [ extraToml.package ])).name or pname;
  };
}
