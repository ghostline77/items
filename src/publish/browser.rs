use crate::model::Catalogue;
use serde_json::{Value, json};

pub(super) fn summary(entry: &Value, script: &str, c: &Catalogue) -> Value {
    let mut out = json!({"id":entry["id"],"detail_script":script});
    for key in [
        "label",
        "native_id",
        "native_name",
        "category",
        "item_type",
        "selections",
        "selection",
        "control",
        "resource",
        "appearance",
        "roles",
        "media",
        "preview_state",
        "kind",
        "parts",
        "assemblies",
    ] {
        if let Some(v) = entry.get(key) {
            out[key] = v.clone();
        }
    }
    if let Some(native) = entry.get("native") {
        out["native"] = json!({"name":native["name"],"ui_slot":native["ui_slot"],"native_class":native["native_class"]});
    }
    if out.get("label").is_none()
        && let Some(appearance) = entry.get("appearance").filter(|v| !v.is_null())
    {
        out["label"] = appearance.clone();
    }
    let mut search = Vec::new();
    if let Some(props) = entry.get("properties") {
        for key in [
            "entityName",
            "appearanceName",
            "appearanceResourceName",
            "equipArea",
            "iconPath",
        ] {
            if let Some(v) = props.get(key) {
                search.push(v.to_string());
            }
        }
    }
    if let Some(data) = entry.get("data") {
        for key in ["name", "names", "morphName", "localizedName"] {
            if let Some(v) = data.get(key) {
                search.push(v.get("$value").unwrap_or(v).to_string());
            }
        }
    }
    if let Some(parts) = entry.get("components").and_then(Value::as_array) {
        let mut terms = std::collections::BTreeSet::new();
        for part in parts {
            for key in ["name", "mesh", "mesh_appearance"] {
                if let Some(v) = part.get(key).filter(|v| !v.is_null()) {
                    terms.insert(v.to_string());
                }
            }
        }
        search.extend(terms);
    }
    if let Some(ids) = entry.get("selections").and_then(Value::as_array) {
        let mut terms = std::collections::BTreeSet::new();
        for id in ids {
            if let Some(s) = id.as_str().and_then(|id| c.selections.get(id)) {
                terms.insert(s.resource.to_string());
                if let Some(a) = &s.appearance {
                    terms.insert(a.clone());
                }
                for part in &s.components {
                    if let Some(mesh) = part.get("mesh") {
                        terms.insert(mesh.to_string());
                    }
                    if let Some(app) = part.get("mesh_appearance") {
                        terms.insert(app.to_string());
                    }
                }
            }
        }
        search.extend(terms);
    }
    if !search.is_empty() {
        out["search_text"] = search.join(" ").to_lowercase().into();
    }
    out
}
