{ pkgs ? import <nixpkgs> {
  overlays = [
    (import (builtins.fetchTarball
      "https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz"))
  ];
} }:
with pkgs;
let

  nightly = ((pkgs.rustChannelOf {
    date = "2019-09-01";
    channel = "nightly";
  }).rust.override {
    extensions = [
      "clippy-preview"
      "rls-preview"
      "rust-analysis"
      "rust-src"
      "rustfmt-preview"
    ];
  });

  nu-deps = [ openssl_1_1 pkg-config x11 python3 ];

  rust = [ nightly rustracer cargo-watch ];

in stdenv.mkDerivation {
  name = "nushell-rust";
  buildInputs = nu-deps ++ rust;
  RUST_SRC_PATH = "${nightly}/lib/rustlib/src/rust/src";
  SSL_CERT_FILE = "/etc/ssl/certs/ca-certificates.crt";
}
