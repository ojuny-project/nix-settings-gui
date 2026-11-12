use crate::dsl::{OptionSpec, WidgetKind};

/// The editor shape of one output feature.
#[derive(Clone, Copy, Debug)]
pub(crate) enum FeatureControl {
    Toggle,
    Text,
    Path,
    Color,
    Spinner {
        min: f64,
        max: f64,
        step: f64,
        digits: u32,
        unit: Option<&'static str>,
    },
    Dropdown(&'static [&'static str]),
    NumberDropdown(&'static [i64]),
}

/// One key in the output contract. `group` is empty for a top-level key and
/// otherwise names a group in `GROUPS`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Feature {
    pub(crate) path: &'static str,
    pub(crate) label: &'static str,
    pub(crate) group: &'static str,
    pub(crate) control: FeatureControl,
    pub(crate) default: &'static str,
    pub(crate) description: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct FeatureGroup {
    pub(crate) key: &'static str,
    pub(crate) label: &'static str,
}

pub(crate) const GROUPS: &[FeatureGroup] = &[
    FeatureGroup {
        key: "match",
        label: "Identity",
    },
    FeatureGroup {
        key: "color",
        label: "Color",
    },
    FeatureGroup {
        key: "hdr",
        label: "HDR",
    },
    FeatureGroup {
        key: "sync",
        label: "Sync",
    },
    FeatureGroup {
        key: "render",
        label: "Rendering",
    },
    FeatureGroup {
        key: "signal",
        label: "Signal",
    },
    FeatureGroup {
        key: "reserved",
        label: "Reserved area",
    },
    FeatureGroup {
        key: "hotCorners",
        label: "Hot corners",
    },
    FeatureGroup {
        key: "background",
        label: "Background",
    },
];

const TRANSFORMS: &[&str] = &[
    "normal",
    "90",
    "180",
    "270",
    "flipped",
    "flipped-90",
    "flipped-180",
    "flipped-270",
];

pub(crate) const FEATURES: &[Feature] = &[
    Feature {
        path: "width",
        label: "Width",
        group: "",
        control: FeatureControl::Spinner {
            min: 0.0,
            max: 16384.0,
            step: 1.0,
            digits: 0,
            unit: Some("px"),
        },
        default: "0",
        description: "Horizontal resolution in pixels. 0 leaves the mode unset.",
    },
    Feature {
        path: "height",
        label: "Height",
        group: "",
        control: FeatureControl::Spinner {
            min: 0.0,
            max: 16384.0,
            step: 1.0,
            digits: 0,
            unit: Some("px"),
        },
        default: "0",
        description: "Vertical resolution in pixels. 0 leaves the mode unset.",
    },
    Feature {
        path: "refresh",
        label: "Refresh",
        group: "",
        control: FeatureControl::Spinner {
            min: 0.0,
            max: 1000.0,
            step: 0.01,
            digits: 3,
            unit: Some("Hz"),
        },
        default: "60.0",
        description: "Mode refresh rate in Hz.",
    },
    Feature {
        path: "xPos",
        label: "X position",
        group: "",
        control: FeatureControl::Spinner {
            min: -16384.0,
            max: 16384.0,
            step: 1.0,
            digits: 0,
            unit: Some("px"),
        },
        default: "0",
        description: "Horizontal position in the logical layout.",
    },
    Feature {
        path: "yPos",
        label: "Y position",
        group: "",
        control: FeatureControl::Spinner {
            min: -16384.0,
            max: 16384.0,
            step: 1.0,
            digits: 0,
            unit: Some("px"),
        },
        default: "0",
        description: "Vertical position in the logical layout.",
    },
    Feature {
        path: "scale",
        label: "Scale",
        group: "",
        control: FeatureControl::Spinner {
            min: 0.01,
            max: 16.0,
            step: 0.05,
            digits: 2,
            unit: None,
        },
        default: "1.0",
        description: "Logical scale factor.",
    },
    Feature {
        path: "transform",
        label: "Transform",
        group: "",
        control: FeatureControl::Dropdown(TRANSFORMS),
        default: "\"normal\"",
        description: "Rotation and flip transform.",
    },
    Feature {
        path: "enabled",
        label: "Enabled",
        group: "",
        control: FeatureControl::Toggle,
        default: "true",
        description: "Whether the output is part of the layout.",
    },
    Feature {
        path: "primary",
        label: "Primary",
        group: "",
        control: FeatureControl::Toggle,
        default: "false",
        description: "Mark the output as the primary display.",
    },
    Feature {
        path: "mirror",
        label: "Mirror output",
        group: "",
        control: FeatureControl::Text,
        default: "\"\"",
        description: "Output name to mirror, empty for none.",
    },
    Feature {
        path: "priority",
        label: "Priority",
        group: "",
        control: FeatureControl::Spinner {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            digits: 0,
            unit: None,
        },
        default: "0",
        description: "Output priority used by compositors that order outputs.",
    },
    Feature {
        path: "customMode",
        label: "Custom mode",
        group: "",
        control: FeatureControl::Toggle,
        default: "false",
        description: "Treat the width, height, and refresh as a custom mode.",
    },
    Feature {
        path: "modeline",
        label: "Modeline",
        group: "",
        control: FeatureControl::Text,
        default: "\"\"",
        description: "Raw modeline for compositors that accept one.",
    },
    Feature {
        path: "power",
        label: "Power",
        group: "",
        control: FeatureControl::Toggle,
        default: "true",
        description: "Power the output without removing its workspaces.",
    },
    Feature {
        path: "reflection",
        label: "Reflection",
        group: "",
        control: FeatureControl::Dropdown(&["none", "horizontal", "vertical", "both"]),
        default: "\"none\"",
        description: "Reflection applied to the output.",
    },
    Feature {
        path: "focusAtStartup",
        label: "Focus at startup",
        group: "",
        control: FeatureControl::Toggle,
        default: "false",
        description: "Focus this output when the session starts.",
    },
    Feature {
        path: "brightness",
        label: "Brightness",
        group: "",
        control: FeatureControl::Spinner {
            min: 0.0,
            max: 2.0,
            step: 0.01,
            digits: 2,
            unit: None,
        },
        default: "1.0",
        description: "Output brightness multiplier.",
    },
    Feature {
        path: "autoRotation",
        label: "Auto rotation",
        group: "",
        control: FeatureControl::Dropdown(&["automatic", "never"]),
        default: "\"never\"",
        description: "Automatic rotation policy for convertible outputs.",
    },
    Feature {
        path: "autoHdr",
        label: "Auto HDR",
        group: "",
        control: FeatureControl::Toggle,
        default: "false",
        description: "Let the compositor decide HDR automatically.",
    },
    Feature {
        path: "match.make",
        label: "Make",
        group: "match",
        control: FeatureControl::Text,
        default: "\"\"",
        description: "Monitor make used for matching.",
    },
    Feature {
        path: "match.model",
        label: "Model",
        group: "match",
        control: FeatureControl::Text,
        default: "\"\"",
        description: "Monitor model used for matching.",
    },
    Feature {
        path: "match.serial",
        label: "Serial",
        group: "match",
        control: FeatureControl::Text,
        default: "\"\"",
        description: "Monitor serial used for matching.",
    },
    Feature {
        path: "color.profile",
        label: "Profile",
        group: "color",
        control: FeatureControl::Dropdown(&[
            "default",
            "srgb",
            "gamma22",
            "wide",
            "hdr",
            "automatic",
            "edid",
            "icc",
        ]),
        default: "\"default\"",
        description: "Color profile preset.",
    },
    Feature {
        path: "color.icc",
        label: "ICC profile",
        group: "color",
        control: FeatureControl::Path,
        default: "\"\"",
        description: "Path to an ICC profile.",
    },
    Feature {
        path: "color.wideGamut",
        label: "Wide gamut",
        group: "color",
        control: FeatureControl::Dropdown(&["auto", "on", "off"]),
        default: "\"auto\"",
        description: "Force wide color gamut support.",
    },
    Feature {
        path: "color.bitDepth",
        label: "Bit depth",
        group: "color",
        control: FeatureControl::NumberDropdown(&[6, 8, 10, 12, 14, 16]),
        default: "8",
        description: "Rendering or signal bit depth.",
    },
    Feature {
        path: "hdr.enabled",
        label: "HDR",
        group: "hdr",
        control: FeatureControl::Toggle,
        default: "false",
        description: "Enable high dynamic range output.",
    },
    Feature {
        path: "hdr.force",
        label: "Force HDR",
        group: "hdr",
        control: FeatureControl::Toggle,
        default: "false",
        description: "Skip the output capability checks for HDR.",
    },
    Feature {
        path: "hdr.minLuminance",
        label: "Min luminance",
        group: "hdr",
        control: FeatureControl::Spinner {
            min: 0.0,
            max: 10000.0,
            step: 1.0,
            digits: 2,
            unit: Some("cd/m2"),
        },
        default: "0.0",
        description: "Minimum mastering luminance.",
    },
    Feature {
        path: "hdr.maxLuminance",
        label: "Max luminance",
        group: "hdr",
        control: FeatureControl::Spinner {
            min: 0.0,
            max: 10000.0,
            step: 1.0,
            digits: 2,
            unit: Some("cd/m2"),
        },
        default: "0.0",
        description: "Maximum mastering luminance.",
    },
    Feature {
        path: "hdr.maxAvgLuminance",
        label: "Max average luminance",
        group: "hdr",
        control: FeatureControl::Spinner {
            min: 0.0,
            max: 10000.0,
            step: 1.0,
            digits: 2,
            unit: Some("cd/m2"),
        },
        default: "0.0",
        description: "Maximum average mastering luminance.",
    },
    Feature {
        path: "hdr.sdrBrightness",
        label: "SDR brightness",
        group: "hdr",
        control: FeatureControl::Spinner {
            min: 0.0,
            max: 10.0,
            step: 0.01,
            digits: 2,
            unit: None,
        },
        default: "1.0",
        description: "SDR brightness in HDR mode.",
    },
    Feature {
        path: "hdr.sdrSaturation",
        label: "SDR saturation",
        group: "hdr",
        control: FeatureControl::Spinner {
            min: 0.0,
            max: 10.0,
            step: 0.01,
            digits: 2,
            unit: None,
        },
        default: "1.0",
        description: "SDR saturation in HDR mode.",
    },
    Feature {
        path: "hdr.sdrMinLuminance",
        label: "SDR min luminance",
        group: "hdr",
        control: FeatureControl::Spinner {
            min: 0.0,
            max: 10000.0,
            step: 0.01,
            digits: 2,
            unit: Some("cd/m2"),
        },
        default: "0.0",
        description: "SDR minimum luminance for the SDR to HDR mapping.",
    },
    Feature {
        path: "hdr.sdrMaxLuminance",
        label: "SDR max luminance",
        group: "hdr",
        control: FeatureControl::Spinner {
            min: 0.0,
            max: 10000.0,
            step: 1.0,
            digits: 2,
            unit: Some("cd/m2"),
        },
        default: "0.0",
        description: "SDR maximum luminance for the SDR to HDR mapping.",
    },
    Feature {
        path: "hdr.sdrEotf",
        label: "SDR EOTF",
        group: "hdr",
        control: FeatureControl::Dropdown(&["default", "gamma22", "srgb"]),
        default: "\"default\"",
        description: "SDR transfer function in HDR mode.",
    },
    Feature {
        path: "sync.vrr",
        label: "VRR",
        group: "sync",
        control: FeatureControl::Dropdown(&[
            "off",
            "on",
            "on-demand",
            "automatic",
            "always",
            "never",
        ]),
        default: "\"off\"",
        description: "Variable refresh rate policy.",
    },
    Feature {
        path: "sync.allowTearing",
        label: "Allow tearing",
        group: "sync",
        control: FeatureControl::Toggle,
        default: "false",
        description: "Allow immediate page flips for fullscreen clients.",
    },
    Feature {
        path: "sync.maxRenderTime",
        label: "Max render time",
        group: "sync",
        control: FeatureControl::Spinner {
            min: 0.0,
            max: 1000.0,
            step: 1.0,
            digits: 0,
            unit: Some("ms"),
        },
        default: "0",
        description: "Compositing lead time before the next refresh.",
    },
    Feature {
        path: "render.scaleFilter",
        label: "Scale filter",
        group: "render",
        control: FeatureControl::Dropdown(&["linear", "nearest", "smart"]),
        default: "\"smart\"",
        description: "Filter used for scaled application buffers.",
    },
    Feature {
        path: "render.subpixel",
        label: "Subpixel",
        group: "render",
        control: FeatureControl::Dropdown(&["rgb", "bgr", "vrgb", "vbgr", "none"]),
        default: "\"none\"",
        description: "Subpixel geometry override.",
    },
    Feature {
        path: "signal.overscan",
        label: "Overscan",
        group: "signal",
        control: FeatureControl::Spinner {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            digits: 0,
            unit: Some("%"),
        },
        default: "0",
        description: "Overscan compensation.",
    },
    Feature {
        path: "signal.rgbRange",
        label: "RGB range",
        group: "signal",
        control: FeatureControl::Dropdown(&["automatic", "full", "limited"]),
        default: "\"automatic\"",
        description: "RGB quantization range.",
    },
    Feature {
        path: "reserved.top",
        label: "Top",
        group: "reserved",
        control: FeatureControl::Spinner {
            min: 0.0,
            max: 1000.0,
            step: 1.0,
            digits: 0,
            unit: Some("px"),
        },
        default: "0",
        description: "Reserved area at the top edge.",
    },
    Feature {
        path: "reserved.right",
        label: "Right",
        group: "reserved",
        control: FeatureControl::Spinner {
            min: 0.0,
            max: 1000.0,
            step: 1.0,
            digits: 0,
            unit: Some("px"),
        },
        default: "0",
        description: "Reserved area at the right edge.",
    },
    Feature {
        path: "reserved.bottom",
        label: "Bottom",
        group: "reserved",
        control: FeatureControl::Spinner {
            min: 0.0,
            max: 1000.0,
            step: 1.0,
            digits: 0,
            unit: Some("px"),
        },
        default: "0",
        description: "Reserved area at the bottom edge.",
    },
    Feature {
        path: "reserved.left",
        label: "Left",
        group: "reserved",
        control: FeatureControl::Spinner {
            min: 0.0,
            max: 1000.0,
            step: 1.0,
            digits: 0,
            unit: Some("px"),
        },
        default: "0",
        description: "Reserved area at the left edge.",
    },
    Feature {
        path: "hotCorners.topLeft",
        label: "Top left",
        group: "hotCorners",
        control: FeatureControl::Toggle,
        default: "false",
        description: "Enable the top-left hot corner.",
    },
    Feature {
        path: "hotCorners.topRight",
        label: "Top right",
        group: "hotCorners",
        control: FeatureControl::Toggle,
        default: "false",
        description: "Enable the top-right hot corner.",
    },
    Feature {
        path: "hotCorners.bottomLeft",
        label: "Bottom left",
        group: "hotCorners",
        control: FeatureControl::Toggle,
        default: "false",
        description: "Enable the bottom-left hot corner.",
    },
    Feature {
        path: "hotCorners.bottomRight",
        label: "Bottom right",
        group: "hotCorners",
        control: FeatureControl::Toggle,
        default: "false",
        description: "Enable the bottom-right hot corner.",
    },
    Feature {
        path: "background.image",
        label: "Image",
        group: "background",
        control: FeatureControl::Path,
        default: "\"\"",
        description: "Background image for this output.",
    },
    Feature {
        path: "background.color",
        label: "Color",
        group: "background",
        control: FeatureControl::Color,
        default: "\"\"",
        description: "Background color for this output.",
    },
    Feature {
        path: "background.backdropColor",
        label: "Backdrop color",
        group: "background",
        control: FeatureControl::Color,
        default: "\"\"",
        description: "Backdrop color behind the workspace.",
    },
    Feature {
        path: "background.mode",
        label: "Mode",
        group: "background",
        control: FeatureControl::Dropdown(&["stretch", "fill", "fit", "center", "tile"]),
        default: "\"fill\"",
        description: "Background image scaling mode.",
    },
];

pub(crate) fn find(path: &str) -> Option<&'static Feature> {
    FEATURES.iter().find(|feature| feature.path == path)
}

pub(crate) fn group(key: &str) -> Option<&'static FeatureGroup> {
    GROUPS.iter().find(|group| group.key == key)
}

pub(crate) fn is_group(key: &str) -> bool {
    group(key).is_some()
}

pub(crate) fn default_for(path: &str) -> &'static str {
    find(path).map(|feature| feature.default).unwrap_or("null")
}

pub(crate) fn features_in_group(key: &str) -> impl Iterator<Item = &'static Feature> + '_ {
    FEATURES.iter().filter(move |feature| feature.group == key)
}

pub(crate) fn spec(feature: &Feature) -> OptionSpec {
    let widget = match feature.control {
        FeatureControl::Toggle => WidgetKind::Switch,
        FeatureControl::Text => WidgetKind::Text,
        FeatureControl::Path => WidgetKind::Path,
        FeatureControl::Color => WidgetKind::Color,
        FeatureControl::Spinner { .. } => WidgetKind::Spin,
        FeatureControl::Dropdown(_) | FeatureControl::NumberDropdown(_) => WidgetKind::Dropdown,
    };
    let mut spec = OptionSpec {
        widget: Some(widget),
        label: Some(feature.label.to_string()),
        description: Some(feature.description.to_string()),
        ..OptionSpec::default()
    };
    match feature.control {
        FeatureControl::Spinner {
            min,
            max,
            step,
            digits,
            unit,
        } => {
            spec.min = Some(min);
            spec.max = Some(max);
            spec.step = Some(step);
            spec.digits = Some(digits);
            spec.unit = unit.map(str::to_string);
        }
        FeatureControl::Dropdown(choices) => {
            spec.choices = Some(choices.iter().map(|choice| (*choice).to_string()).collect());
        }
        FeatureControl::NumberDropdown(values) => {
            spec.choices = Some(values.iter().map(|value| value.to_string()).collect());
        }
        _ => {}
    }
    spec
}

/// Resolved `disabledFeatures` entries. A parent group disables every child; an
/// exact dotted path disables only that child.
#[derive(Clone, Debug, Default)]
pub(crate) struct DisabledSet {
    paths: Vec<String>,
}

impl DisabledSet {
    pub(crate) fn from_paths(paths: &[String]) -> Result<Self, String> {
        let mut normalized = Vec::new();
        for path in paths {
            let path = path.trim();
            if path.is_empty() {
                continue;
            }
            if !is_group(path) && find(path).is_none() {
                return Err(format!(
                    "disabledFeatures entry '{path}' is not a known output feature"
                ));
            }
            normalized.push(path.to_string());
        }
        Ok(Self { paths: normalized })
    }

    pub(crate) fn contains(&self, path: &str) -> bool {
        if self.paths.iter().any(|entry| entry == path) {
            return true;
        }
        match path.split_once('.') {
            Some((parent, _)) => self.paths.iter().any(|entry| entry == parent),
            None => false,
        }
    }

    pub(crate) fn group_visible(&self, key: &str) -> bool {
        !self.contains(key)
    }

    pub(crate) fn any_visible(&self, key: &str) -> bool {
        self.group_visible(key)
            && features_in_group(key).any(|feature| !self.contains(feature.path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_path_is_unique() {
        for (index, feature) in FEATURES.iter().enumerate() {
            assert!(
                !FEATURES[index + 1..]
                    .iter()
                    .any(|other| other.path == feature.path),
                "duplicate path: {}",
                feature.path
            );
        }
    }

    #[test]
    fn every_group_has_a_feature() {
        for group in GROUPS {
            assert!(
                FEATURES.iter().any(|feature| feature.group == group.key),
                "group without features: {}",
                group.key
            );
        }
    }

    #[test]
    fn parent_disables_children_but_child_does_not_disable_parent() {
        let disabled = DisabledSet::from_paths(&["hdr".to_string()]).unwrap();
        assert!(disabled.contains("hdr"));
        assert!(disabled.contains("hdr.sdrEotf"));
        assert!(!disabled.contains("color.icc"));

        let disabled = DisabledSet::from_paths(&["hdr.sdrEotf".to_string()]).unwrap();
        assert!(!disabled.contains("hdr"));
        assert!(disabled.contains("hdr.sdrEotf"));
        assert!(!disabled.contains("hdr.enabled"));
    }

    #[test]
    fn unknown_paths_are_rejected() {
        let error = DisabledSet::from_paths(&["hdr.nope".to_string()]).unwrap_err();
        assert!(error.contains("hdr.nope"));
    }

    #[test]
    fn disable_keys_use_the_group_for_children() {
        let disabled = DisabledSet::from_paths(&["hdr.sdrEotf".to_string()]).unwrap();
        assert!(disabled.contains("hdr.sdrEotf"));
        assert!(!disabled.contains("background.image"));
    }
}
