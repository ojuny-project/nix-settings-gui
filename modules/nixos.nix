self: {
  config,
  lib,
  pkgs,
  ...
}:
with lib; let
  cfg = config.services.nix-settings-gui;
in {
  imports = [ ./monitors.nix ];

  options.services.nix-settings-gui = {
    enable = mkEnableOption "nix-settings-gui, a comment-driven settings GUI for Nix";

    withOjunyWidgets = mkOption {
      type = types.bool;
      default = true;
      description = "Build with the ojuny-specific bar, dock, menu, plugins, grid and docker widgets.";
    };

    package = mkOption {
      type = types.package;
      default =
        if cfg.withOjunyWidgets
        then self.packages.${pkgs.stdenv.hostPlatform.system}.default
        else self.packages.${pkgs.stdenv.hostPlatform.system}.nix-settings-gui-minimal;
      description = "The nix-settings-gui package to use.";
    };
  };

  config = mkIf cfg.enable {
    environment.systemPackages = [cfg.package];
  };
}
