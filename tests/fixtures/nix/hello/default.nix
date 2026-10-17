{ lib, stdenv, fetchurl }:
stdenv.mkDerivation {
  pname = "hello";
  version = "1.0";
  src = import ./src.nix;
  meta.description = "example package";
}
