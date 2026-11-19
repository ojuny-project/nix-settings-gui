/// Card surfaces whose catalog defaults differ, and the geometry a generic
/// grid uses when no catalog entry exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GridSurface {
    ControlCenter,
    Dashboard,
    Weather,
    Generic,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CardSize {
    pub(crate) w: i64,
    pub(crate) h: i64,
    pub(crate) min_w: i64,
    pub(crate) min_h: i64,
    pub(crate) max_w: i64,
    pub(crate) max_h: i64,
}

#[allow(dead_code)]
pub(crate) fn default_size() -> CardSize {
    CardSize {
        w: 1,
        h: 1,
        min_w: 1,
        min_h: 1,
        max_w: 64,
        max_h: 64,
    }
}

/// Default and bounds for a card id. With the ojuny feature the DMS catalog
/// answers; otherwise every id gets the generic 1x1 scaffold.
pub(crate) fn card_size(surface: GridSurface, id: &str) -> CardSize {
    #[cfg(feature = "ojuny-widgets")]
    {
        super::dms_catalog::card_size(surface, id)
    }
    #[cfg(not(feature = "ojuny-widgets"))]
    {
        let _ = (surface, id);
        default_size()
    }
}

/// Option defaults scaffolded when a card is added.
pub(crate) fn card_defaults(surface: GridSurface, id: &str) -> Vec<(&'static str, &'static str)> {
    #[cfg(feature = "ojuny-widgets")]
    {
        super::dms_catalog::card_defaults(surface, id)
    }
    #[cfg(not(feature = "ojuny-widgets"))]
    {
        let _ = (surface, id);
        Vec::new()
    }
}

pub(crate) fn widget_icon(id: &str) -> &'static str {
    #[cfg(feature = "ojuny-widgets")]
    {
        super::dms_catalog::widget_icon(id)
    }
    #[cfg(not(feature = "ojuny-widgets"))]
    {
        let _ = id;
        "application-x-executable-symbolic"
    }
}

pub(crate) fn widget_description(id: &str) -> &'static str {
    #[cfg(feature = "ojuny-widgets")]
    {
        super::dms_catalog::widget_description(id)
    }
    #[cfg(not(feature = "ojuny-widgets"))]
    {
        let _ = id;
        ""
    }
}

/// One grid surface and the card ids its picker offers.
pub(crate) struct GridProfileData {
    pub(crate) surface: GridSurface,
    pub(crate) key: &'static str,
    pub(crate) label: &'static str,
    pub(crate) columns: i64,
    pub(crate) max_rows: i64,
    pub(crate) has_enabled: bool,
    pub(crate) registry: Vec<String>,
}

fn profile_data(
    profile: super::grid::GridProfile,
    key: &'static str,
    label: &'static str,
) -> GridProfileData {
    GridProfileData {
        surface: profile.surface,
        key,
        label,
        columns: profile.columns,
        max_rows: profile.max_rows,
        has_enabled: profile.has_enabled,
        registry: profile.registry.iter().cloned().collect(),
    }
}

/// Every surface the editor can build, including the generic fallback.
pub(crate) fn profile_metadata() -> Vec<GridProfileData> {
    let mut profiles = Vec::new();
    #[cfg(feature = "ojuny-widgets")]
    {
        if let Some(profile) =
            super::grid::GridProfile::for_path("programs.ojuny.dux.controlCenterWidgets")
        {
            profiles.push(profile_data(profile, "controlCenter", "Control Center"));
        }
        if let Some(profile) = super::grid::GridProfile::for_path("desktop.ojuny.dux.dashCards") {
            profiles.push(profile_data(profile, "dashboard", "Dashboard"));
        }
        if let Some(profile) =
            super::grid::GridProfile::for_path("programs.ojuny.dux.weather.widgets")
        {
            profiles.push(profile_data(profile, "weather", "Weather"));
        }
    }
    profiles.push(profile_data(
        super::grid::GridProfile::generic(),
        "generic",
        "Generic",
    ));
    profiles
}

/// Default sizes for every card id the profiles know about.
pub(crate) fn size_metadata() -> Vec<CardSize> {
    let mut sizes = Vec::new();
    for profile in profile_metadata() {
        for id in &profile.registry {
            sizes.push(card_size(profile.surface, id));
        }
    }
    if sizes.is_empty() {
        sizes.push(default_size());
    }
    sizes
}
