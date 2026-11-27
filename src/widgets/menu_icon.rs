pub(crate) const FALLBACK_MENU_ICON: &str = "emblem-system-symbolic";

const MENU_ICON_ALTERNATIVES: &[(&str, &[&str])] = &[
    ("preferences-system-symbolic", &["emblem-system-symbolic"]),
    ("applications-system-symbolic", &["emblem-system-symbolic"]),
    ("preferences-other-symbolic", &["emblem-system-symbolic"]),
];

pub(crate) fn pick_icon_name(
    has_icon: impl Fn(&str) -> bool,
    icon: Option<&str>,
) -> Option<String> {
    let name = icon?;
    if has_icon(name) {
        return Some(name.to_string());
    }
    for (missing, alternatives) in MENU_ICON_ALTERNATIVES {
        if *missing == name {
            for alternative in *alternatives {
                if has_icon(alternative) {
                    return Some(alternative.to_string());
                }
            }
        }
    }
    if has_icon(FALLBACK_MENU_ICON) {
        return Some(FALLBACK_MENU_ICON.to_string());
    }
    None
}
