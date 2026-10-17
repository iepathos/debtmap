{ config, lib, pkgs, ... }:
let
  mkService = { enable ? false }:
    if enable then
      if config.networking.hostName == "test" then pkgs.hello else pkgs.coreutils
    else
      null;
in
{
  options.services.example.enable = lib.mkEnableOption "example";
  config = lib.mkIf config.services.example.enable {
    environment.systemPackages = [ (mkService { enable = true; }) ];
  };
}
