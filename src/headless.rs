//! Read-only metadata for non-GTK frontends.

use serde_json::{json, Value};

/// Provider choices for the menu widget. `top_level` hides Apps List, which is
/// only valid on a top-level menu entry.
pub fn menu_providers(top_level: bool) -> Value {
    let providers: Vec<Value> = crate::widgets::names::MENU_PROVIDERS
        .iter()
        .filter(|choice| top_level || choice.provider != "apps")
        .map(|choice| {
            json!({
                "label": choice.label,
                "provider": choice.provider,
                "fields": choice.fields,
            })
        })
        .collect();
    Value::Array(providers)
}

pub fn menu_field_help(name: &str) -> Option<&'static str> {
    crate::widgets::names::menu_field_help(name)
}

pub fn keybind_field_help(name: &str) -> Option<&'static str> {
    crate::widgets::names::keybind_field_help(name)
}

/// Default card sizes across the grid surfaces, sorted by height then width.
pub fn grid_card_sizes() -> Value {
    let mut sizes = crate::widgets::grid_catalog::size_metadata();
    sizes.sort_by_key(|size| (size.h, size.w));
    sizes.dedup_by_key(|size| (size.w, size.h));
    Value::Array(
        sizes
            .iter()
            .map(|size| {
                json!({
                    "w": size.w,
                    "h": size.h,
                    "label": format!("{}x{}", size.w, size.h),
                })
            })
            .collect(),
    )
}

/// Every grid surface with its column count, row limit, and registry cards.
pub fn grid_profiles() -> Value {
    let profiles: Vec<Value> = crate::widgets::grid_catalog::profile_metadata()
        .into_iter()
        .map(|profile| {
            let registry: Vec<Value> = profile
                .registry
                .iter()
                .map(|id| {
                    json!({
                        "id": id,
                        "label": crate::widgets::names::widget_display_name(id),
                    })
                })
                .collect();
            json!({
                "surface": profile.key,
                "label": profile.label,
                "columns": profile.columns,
                "maxRows": profile.max_rows,
                "hasEnabled": profile.has_enabled,
                "registry": registry,
            })
        })
        .collect();
    Value::Array(profiles)
}

/// The controls declared by a plugin's Settings.qml.
#[cfg(feature = "ojuny-widgets")]
pub fn plugin_settings_schema(qml: &str) -> Value {
    use crate::plugin_schema::{parse_settings, SchemaKind};
    let fields: Vec<Value> = parse_settings(qml)
        .into_iter()
        .map(|field| {
            let (kind, choices) = match &field.kind {
                SchemaKind::Bool => ("bool", Vec::new()),
                SchemaKind::Int => ("int", Vec::new()),
                SchemaKind::Float => ("float", Vec::new()),
                SchemaKind::Text => ("text", Vec::new()),
                SchemaKind::Dropdown(choices) => ("dropdown", choices.clone()),
                SchemaKind::Color => ("color", Vec::new()),
                SchemaKind::Raw => ("raw", Vec::new()),
            };
            json!({
                "key": field.key,
                "label": field.label,
                "description": field.description,
                "kind": kind,
                "choices": choices,
                "default": field.default,
                "minimum": field.minimum,
                "maximum": field.maximum,
            })
        })
        .collect();
    Value::Array(fields)
}

#[cfg(all(test, feature = "ojuny-widgets"))]
mod tests {
    use super::*;

    #[test]
    fn grid_metadata_is_populated_with_the_dms_catalog() {
        let sizes = grid_card_sizes();
        let profiles = grid_profiles();
        assert!(!sizes.as_array().unwrap().is_empty());
        assert!(!profiles.as_array().unwrap().is_empty());
    }
}
