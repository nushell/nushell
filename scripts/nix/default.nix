# Generic builder for nushell and its included plugins.
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
  # Mostly args and things that might be different for plugins
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

  # Fixed in #16914
  buildAndTestSubdir ? ".",
  checkFlags ? [ ],
  buildType ? "release",
  checkType ? buildType,
  ...
}@args:
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
    checkFlags
    buildType
    checkType
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

  # Builds the plugins for the test using the right profile, otherwise the
  # plugins cannot be found.
  NUSHELL_CARGO_PROFILE = checkType;
  NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8";
  preCheck = ''
    export HOME=$(mktemp -d)
  '';
  meta = {
    description = extraToml.package.description or cargoToml.package.description;
    homepage =
      if cargoToml == extraToml then cargoToml.package.homepage else extraToml.package.repository or null;
    license = lib.licenses.mit;
    mainProgram = (head (extraToml.bin or [ extraToml.package ])).name or pname;
  };
}
