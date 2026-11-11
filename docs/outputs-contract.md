# Outputs contract

`services.nix-settings-gui.monitors` is an attribute set keyed by output name.
The key is the output identity. It cannot be renamed in the widget; replace an
output by removing the old key and adding the new one.

The contract is available in the NixOS and Home Manager modules shipped by
nix-settings-gui. It describes display capabilities only. A compositor adapter
maps the values to Mango, Hyprland, GNOME, sway, niri, KDE Plasma, or XFCE
configuration.

## Attachment

```nix
/* @option widget = "monitors"; label = "Monitors"; disabledFeatures = [ "background" ]; */
services.nix-settings-gui.monitors = {
  "eDP-1" = {
    width = 1920;
    height = 1080;
    refresh = 60;
    xPos = 0;
    yPos = 0;
    scale = 1.0;
    transform = "normal";
  };
};
```

The annotated assignment must be a literal attribute set. Do not use `map`,
`forEach`, `lib.optionalAttrs`, or another function call, per
`ojuny-template/docs/05-settings-gui-limitations.md`.

## Keys

Core keys:

| Key | Type | Default |
|---|---|---|
| `width` | int | `0` |
| `height` | int | `0` |
| `refresh` | number | `60.0` |
| `xPos` | int | `0` |
| `yPos` | int | `0` |
| `scale` | positive number | `1.0` |
| `transform` | enum | `normal` |
| `enabled` | bool | `true` |
| `primary` | bool | `false` |
| `mirror` | string | `""` |
| `priority` | unsigned int | `0` |
| `customMode` | bool | `false` |
| `modeline` | string | `""` |
| `power` | bool | `true` |
| `reflection` | enum | `none` |
| `focusAtStartup` | bool | `false` |
| `brightness` | number | `1.0` |
| `autoRotation` | enum | `never` |
| `autoHdr` | bool | `false` |

Grouped keys:

| Group | Keys |
|---|---|
| `match` | `make`, `model`, `serial` |
| `color` | `profile`, `icc`, `wideGamut`, `bitDepth` |
| `hdr` | `enabled`, `force`, `minLuminance`, `maxLuminance`, `maxAvgLuminance`, `sdrBrightness`, `sdrSaturation`, `sdrMinLuminance`, `sdrMaxLuminance`, `sdrEotf` |
| `sync` | `vrr`, `allowTearing`, `maxRenderTime` |
| `render` | `scaleFilter`, `subpixel` |
| `signal` | `overscan`, `rgbRange` |
| `reserved` | `top`, `right`, `bottom`, `left` |
| `hotCorners` | `topLeft`, `topRight`, `bottomLeft`, `bottomRight` |
| `background` | `image`, `color`, `backdropColor`, `mode` |

`match.*` identifies the display and does not change it. `autoHdr` is separate
from `hdr.enabled` so a consumer can hide only the automatic HDR choice while
keeping the HDR group.

## disabledFeatures

Entries are paths relative to `monitors.<name>`:

```nix
disabledFeatures = [ "width" "hdr" "hdr.sdrEotf" "color.icc" ];
```

- `width` disables a top-level key.
- `hdr` disables the whole group.
- `hdr.sdrEotf` disables one child and leaves the rest of `hdr` visible.
- Unknown paths are an error. A typo cannot silently leave a field enabled.
- If a parent and a child are both listed, the parent wins.
- A group whose children are all disabled is hidden.
- Disabled keys are omitted from the value written on save.

## Adapters

An adapter reads `config.services.nix-settings-gui.monitors` and writes its own
compositor option. The Mango adapter in ojuny-template adds the global ICC
profile to every generated monitor rule. Other adapters follow the same shape.

## Feature sources

The union covers MangoWM, Hyprland, GNOME, sway, niri, KDE Plasma, and XFCE.
The widget does not implement or validate any compositor syntax.
