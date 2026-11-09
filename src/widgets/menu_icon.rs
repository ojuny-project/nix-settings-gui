pub(crate) const FALLBACK_MENU_ICON: &str = "preferences-system-symbolic";

pub(crate) fn pick_icon_name(
    has_icon: impl Fn(&str) -> bool,
    icon: Option<&str>,
) -> Option<String> {
    let name = icon?;
    if has_icon(name) {
        return Some(name.to_string());
    }
    if has_icon(FALLBACK_MENU_ICON) {
        return Some(FALLBACK_MENU_ICON.to_string());
    }
    None
}
