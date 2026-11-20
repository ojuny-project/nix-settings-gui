//! Picks an icon for a `@menu category`, which has no file and no icon of its
//! own. The catalog maps category keywords to theme icon names; when no rule
//! matches, the most common page icon in the category is used.

pub(crate) const FALLBACK_CATEGORY_ICON: &str = "preferences-system-symbolic";

struct CategoryIconRule {
    icon: &'static str,
    keywords: &'static [&'static str],
}

const CATEGORY_ICON_CATALOG: &[CategoryIconRule] = &[
    CategoryIconRule {
        icon: "system-users-symbolic",
        keywords: &["users", "user", "accounts", "account", "people"],
    },
    CategoryIconRule {
        icon: "preferences-desktop-theme-symbolic",
        keywords: &["gui", "appearance", "theme", "themes", "interface"],
    },
    CategoryIconRule {
        icon: "applications-development-symbolic",
        keywords: &["patches", "patch", "development", "dev"],
    },
    CategoryIconRule {
        icon: "view-app-grid-symbolic",
        keywords: &["applications", "apps", "software"],
    },
    CategoryIconRule {
        icon: "applications-utilities-symbolic",
        keywords: &["utility", "utilities", "tools"],
    },
    CategoryIconRule {
        icon: "preferences-desktop-symbolic",
        keywords: &["desktop", "session", "workspaces", "workspace"],
    },
    CategoryIconRule {
        icon: "preferences-desktop-sound-symbolic",
        keywords: &["audio", "sound", "sounds"],
    },
    CategoryIconRule {
        icon: "network-wireless-symbolic",
        keywords: &["network", "networking", "internet"],
    },
    CategoryIconRule {
        icon: "bluetooth-active-symbolic",
        keywords: &["bluetooth"],
    },
    CategoryIconRule {
        icon: "video-display-symbolic",
        keywords: &["display", "graphics", "monitors"],
    },
    CategoryIconRule {
        icon: "preferences-desktop-keyboard-symbolic",
        keywords: &["input", "keyboard"],
    },
    CategoryIconRule {
        icon: "security-high-symbolic",
        keywords: &["security", "secure", "boot"],
    },
    CategoryIconRule {
        icon: "preferences-system-symbolic",
        keywords: &["system", "core", "general", "settings"],
    },
];

/// Strip a leading `NN-` sort prefix, lowercase, and split on non-alphanumerics.
pub(crate) fn category_tokens(category: &str) -> Vec<String> {
    let trimmed = category
        .trim()
        .trim_start_matches(|c: char| c.is_ascii_digit());
    let text = trimmed.strip_prefix('-').unwrap_or(trimmed);
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

/// Resolve the category icon in this order: catalog keyword match, most common
/// page icon in the category, generic fallback. Every candidate must be
/// reported by `has_icon`.
pub(crate) fn pick_category_icon_name(
    has_icon: impl Fn(&str) -> bool,
    category: &str,
    member_icons: &[Option<&str>],
) -> Option<String> {
    let tokens = category_tokens(category);
    let mut best: Option<(usize, usize, &'static str)> = None;
    for rule in CATEGORY_ICON_CATALOG {
        let score = rule
            .keywords
            .iter()
            .filter(|keyword| tokens.iter().any(|token| token == *keyword))
            .count();
        if score == 0 || !has_icon(rule.icon) {
            continue;
        }
        let specificity = rule.keywords.len();
        let better = best.is_none_or(|(best_score, best_specificity, _)| {
            score > best_score || (score == best_score && specificity < best_specificity)
        });
        if better {
            best = Some((score, specificity, rule.icon));
        }
    }
    if let Some((_, _, icon)) = best {
        return Some(icon.to_string());
    }

    let mut counts: Vec<(&str, usize)> = Vec::new();
    for icon in member_icons.iter().flatten() {
        if !has_icon(icon) {
            continue;
        }
        match counts.iter_mut().find(|(name, _)| name == icon) {
            Some((_, count)) => *count += 1,
            None => counts.push((icon, 1)),
        }
    }
    let mut best_member: Option<(&str, usize)> = None;
    for (name, count) in counts {
        if best_member.is_none_or(|(_, best_count)| count > best_count) {
            best_member = Some((name, count));
        }
    }
    if let Some((name, _)) = best_member {
        return Some(name.to_string());
    }

    has_icon(FALLBACK_CATEGORY_ICON).then(|| FALLBACK_CATEGORY_ICON.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(_: &str) -> bool {
        true
    }

    #[test]
    fn strips_the_sort_prefix_before_matching() {
        assert_eq!(category_tokens("07-System"), vec!["system"]);
        assert_eq!(category_tokens("01-Users"), vec!["users"]);
        assert_eq!(category_tokens("Patches"), vec!["patches"]);
    }

    #[test]
    fn matches_a_catalog_rule_by_keyword() {
        assert_eq!(
            pick_category_icon_name(all, "07-System", &[]).as_deref(),
            Some("preferences-system-symbolic")
        );
        assert_eq!(
            pick_category_icon_name(all, "01-Users", &[]).as_deref(),
            Some("system-users-symbolic")
        );
        assert_eq!(
            pick_category_icon_name(all, "Patches", &[]).as_deref(),
            Some("applications-development-symbolic")
        );
    }

    #[test]
    fn prefers_the_rule_with_the_most_keyword_hits() {
        assert_eq!(
            pick_category_icon_name(all, "Desktop Sound", &[]).as_deref(),
            Some("preferences-desktop-sound-symbolic")
        );
    }

    #[test]
    fn skips_rules_the_theme_does_not_have() {
        let only_users = |name: &str| name == "system-users-symbolic";
        assert_eq!(
            pick_category_icon_name(only_users, "01-Users", &[]).as_deref(),
            Some("system-users-symbolic")
        );
    }

    #[test]
    fn falls_back_to_the_most_common_member_icon() {
        let members = [
            Some("first-symbolic"),
            Some("second-symbolic"),
            Some("first-symbolic"),
        ];
        assert_eq!(
            pick_category_icon_name(all, "99-Misc", &members).as_deref(),
            Some("first-symbolic")
        );
    }

    #[test]
    fn ignores_invalid_member_icons() {
        let only_valid = |name: &str| name == "valid-symbolic";
        let members = [Some("invalid-symbolic"), Some("valid-symbolic")];
        assert_eq!(
            pick_category_icon_name(only_valid, "99-Misc", &members).as_deref(),
            Some("valid-symbolic")
        );
    }

    #[test]
    fn falls_back_to_the_generic_icon() {
        let only_generic = |name: &str| name == FALLBACK_CATEGORY_ICON;
        assert_eq!(
            pick_category_icon_name(only_generic, "99-Misc", &[]).as_deref(),
            Some(FALLBACK_CATEGORY_ICON)
        );
    }

    #[test]
    fn returns_none_when_the_theme_has_nothing() {
        assert_eq!(pick_category_icon_name(|_| false, "99-Misc", &[]), None);
    }

    #[test]
    fn ties_between_member_icons_keep_the_first_seen() {
        let members = [Some("zeta-symbolic"), Some("alpha-symbolic")];
        assert_eq!(
            pick_category_icon_name(all, "99-Misc", &members).as_deref(),
            Some("zeta-symbolic")
        );
    }
}
