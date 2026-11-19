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
