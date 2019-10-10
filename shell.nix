let
 moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  nixpkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };
  nightly = ((nixpkgs.rustChannelOf { date = "2019-09-01"; channel = "nightly"; }).rust.override { extensions = [ "rust-src" "rls-preview" "clippy-preview" "rust-analysis" "rustfmt-preview" ];});
in
with nixpkgs;
stdenv.mkDerivation {
  name = "nushell-rust";
  buildInputs = [ nightly openssl_1_1 pkg-config ];
}
