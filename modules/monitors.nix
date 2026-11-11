{ lib, ... }:
let
  inherit (lib) mkOption types;

  mkBool = default: description: mkOption {
    type = types.bool;
    inherit default description;
  };
  mkInt = default: description: mkOption {
    type = types.int;
    inherit default description;
  };
  mkUnsigned = default: description: mkOption {
    type = types.ints.unsigned;
    inherit default description;
  };
  mkNumber = default: description: mkOption {
    type = types.numbers.nonnegative;
    inherit default description;
  };
  mkPositive = default: description: mkOption {
    type = types.numbers.positive;
    inherit default description;
  };
  mkStr = default: description: mkOption {
    type = types.str;
    inherit default description;
  };
  mkEnum = values: default: description: mkOption {
    type = types.enum values;
    inherit default description;
  };
  mkGroup = description: options: mkOption {
    type = types.submodule { inherit options; };
    default = { };
    inherit description;
  };
in
{
  options.services.nix-settings-gui.monitors = mkOption {
    type = types.attrsOf (types.submodule {
      options = {
        width = mkInt 0 "Horizontal resolution in pixels. 0 leaves the mode unset.";
        height = mkInt 0 "Vertical resolution in pixels. 0 leaves the mode unset.";
        refresh = mkNumber 60.0 "Mode refresh rate in Hz.";
        xPos = mkInt 0 "Horizontal position in the logical layout.";
        yPos = mkInt 0 "Vertical position in the logical layout.";
        scale = mkPositive 1.0 "Logical scale factor.";
        transform = mkEnum [
          "normal"
          "90"
          "180"
          "270"
          "flipped"
          "flipped-90"
          "flipped-180"
          "flipped-270"
        ] "normal" "Rotation and flip transform.";
        enabled = mkBool true "Whether the output is part of the layout.";
        primary = mkBool false "Mark the output as the primary display.";
        mirror = mkStr "" "Output name to mirror, empty for none.";
        priority = mkUnsigned 0 "Output priority used by compositors that order outputs.";
        customMode = mkBool false "Treat the width, height, and refresh as a custom mode.";
        modeline = mkStr "" "Raw modeline for compositors that accept one.";
        power = mkBool true "Power the output without removing its workspaces.";
        reflection = mkEnum [ "none" "horizontal" "vertical" "both" ] "none" "Reflection applied to the output.";
        focusAtStartup = mkBool false "Focus this output when the session starts.";
        brightness = mkNumber 1.0 "Output brightness multiplier.";
        autoRotation = mkEnum [ "automatic" "never" ] "never" "Automatic rotation policy.";
        autoHdr = mkBool false "Let the compositor decide HDR automatically.";

        match = mkGroup "Identity fields used to match the output instead of, or in addition to, its name." {
          make = mkStr "" "Monitor make.";
          model = mkStr "" "Monitor model.";
          serial = mkStr "" "Monitor serial.";
        };

        color = mkGroup "Color profile and bit depth." {
          profile = mkEnum [
            "default"
            "srgb"
            "gamma22"
            "wide"
            "hdr"
            "automatic"
            "edid"
            "icc"
          ] "default" "Color profile preset.";
          icc = mkStr "" "Path to an ICC profile.";
          wideGamut = mkEnum [ "auto" "on" "off" ] "auto" "Wide color gamut policy.";
          bitDepth = mkEnum [ 6 8 10 12 14 16 ] 8 "Rendering or signal bit depth.";
        };

        hdr = mkGroup "High dynamic range and luminance." {
          enabled = mkBool false "Enable high dynamic range output.";
          force = mkBool false "Skip the output capability checks for HDR.";
          minLuminance = mkNumber 0.0 "Minimum mastering luminance in cd/m2.";
          maxLuminance = mkNumber 0.0 "Maximum mastering luminance in cd/m2.";
          maxAvgLuminance = mkNumber 0.0 "Maximum average mastering luminance in cd/m2.";
          sdrBrightness = mkNumber 1.0 "SDR brightness in HDR mode.";
          sdrSaturation = mkNumber 1.0 "SDR saturation in HDR mode.";
          sdrMinLuminance = mkNumber 0.0 "SDR minimum luminance in cd/m2.";
          sdrMaxLuminance = mkNumber 0.0 "SDR maximum luminance in cd/m2.";
          sdrEotf = mkEnum [ "default" "gamma22" "srgb" ] "default" "SDR transfer function in HDR mode.";
        };

        sync = mkGroup "Variable refresh rate and frame pacing." {
          vrr = mkEnum [
            "off"
            "on"
            "on-demand"
            "automatic"
            "always"
            "never"
          ] "off" "Variable refresh rate policy.";
          allowTearing = mkBool false "Allow immediate page flips for fullscreen clients.";
          maxRenderTime = mkUnsigned 0 "Compositing lead time before the next refresh in milliseconds.";
        };

        render = mkGroup "Scaling and subpixel rendering." {
          scaleFilter = mkEnum [ "linear" "nearest" "smart" ] "smart" "Filter used for scaled application buffers.";
          subpixel = mkEnum [ "rgb" "bgr" "vrgb" "vbgr" "none" ] "none" "Subpixel geometry override.";
        };

        signal = mkGroup "Signal properties." {
          overscan = mkUnsigned 0 "Overscan compensation in percent.";
          rgbRange = mkEnum [ "automatic" "full" "limited" ] "automatic" "RGB quantization range.";
        };

        reserved = mkGroup "Reserved area at the output edges." {
          top = mkUnsigned 0 "Reserved area at the top edge in pixels.";
          right = mkUnsigned 0 "Reserved area at the right edge in pixels.";
          bottom = mkUnsigned 0 "Reserved area at the bottom edge in pixels.";
          left = mkUnsigned 0 "Reserved area at the left edge in pixels.";
        };

        hotCorners = mkGroup "Hot corners for this output." {
          topLeft = mkBool false "Enable the top-left hot corner.";
          topRight = mkBool false "Enable the top-right hot corner.";
          bottomLeft = mkBool false "Enable the bottom-left hot corner.";
          bottomRight = mkBool false "Enable the bottom-right hot corner.";
        };

        background = mkGroup "Background image and color for this output." {
          image = mkStr "" "Background image path.";
          color = mkStr "" "Background color.";
          backdropColor = mkStr "" "Backdrop color behind the workspace.";
          mode = mkEnum [ "stretch" "fill" "fit" "center" "tile" ] "fill" "Background image scaling mode.";
        };
      };
    });
    default = { };
    description = ''
      Generic monitor outputs keyed by connector name. This option describes
      display capabilities only; a compositor adapter maps it to the
      compositor's own configuration.
    '';
  };
}
