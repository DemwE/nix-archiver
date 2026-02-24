# Minimal nix-archiver configuration
# /etc/nixos/configuration.nix

{ config, pkgs, ... }:

{
  imports = [
    /path/to/nix-archiver/modules/nix-archiver.nix
  ];

  # Simply enable and pin one package
  services.nix-archiver = {
    enable = true;
    
    pinnedPackages = {
      nodejs = "20.11.0";
    };
  };

  # nodejs will now use version 20.11.0
  environment.systemPackages = [ pkgs.nodejs ];
}
