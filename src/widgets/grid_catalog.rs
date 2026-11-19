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
