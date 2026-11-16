//! Automatic catalogue binding resolution; no rendering or texture extraction.
use crate::{build::add_asset, model::Catalogue};
use ghostline_red::assets::{Assembly, MeshAppearanceBinding, ResourceKey};
use ghostline_renderer::GameSource;
use serde_json::json;
use std::collections::BTreeMap;

/// Explicit moved-resource corrections; requested native identities remain in context.
/// Both targets are installed resources from the same weapon family. The sledgehammer
/// player resource declares the exact `boss` appearance requested by the legacy entity.
pub(crate) fn resource_overrides() -> BTreeMap<String, ResourceKey> {
    [
        ("12704938643437795714", r"base\weapons\firearms\rifle_precision\techtronika_pozhar\entities\meshes\w_rifle_precision__techtronika_pozhar__mag_stdr_01_decal_01_n_default.mesh"),
        ("13040619209012435006", r"base\weapons\melee\sledge_hammer\tech\w_sledge_hammer_tech_player.app"),
    ].into_iter().map(|(requested, replacement)| (requested.into(), ResourceKey::parse(replacement))).collect()
}

fn collect_overrides(
    value: &serde_json::Value,
    overrides: &BTreeMap<String, ResourceKey>,
    used: &mut BTreeMap<String, ResourceKey>,
) {
    if value.get("DepotPath").is_some()
        && let Ok(Some(key)) = ResourceKey::from_value(value)
        && let Some(replacement) = overrides.get(&key.native_hash().to_string())
    {
        used.insert(key.native_hash().to_string(), replacement.clone());
    }
    match value {
        serde_json::Value::Object(fields) => {
            for child in fields.values() {
                collect_overrides(child, overrides, used);
            }
        }
        serde_json::Value::Array(children) => {
            for child in children {
                collect_overrides(child, overrides, used);
            }
        }
        _ => {}
    }
}

// An empty genital overlay has no independent skin material. Only accept a
// linked palette choice when its existing base definition is literally empty;
// this must never become a general fallback for missing visible appearances.
fn linked_empty_appearance(
    c: &Catalogue,
    selection: &crate::model::Selection,
    reader: &mut ghostline_red::assets::AssetReader<'_>,
) -> Option<String> {
    let requested = selection.appearance.as_deref()?;
    if !requested.starts_with("i0_000_pwa_base__vagina__") {
        return None;
    }
    let linked = c
        .choices
        .iter()
        .filter(|choice| choice.selection.as_deref() == Some(&selection.id))
        .any(|choice| {
            c.controls.iter().any(|control| {
                control.id == choice.control
                    && control.native.data["link"]["$value"] == "skin color"
            })
        });
    if !linked {
        return None;
    }
    let document = reader.document(&selection.resource).ok()?;
    let definitions = document["Data"]["RootChunk"]["appearances"].as_array()?;
    let names: Vec<_> = definitions
        .iter()
        .filter_map(|definition| {
            let definition = definition.get("Data").unwrap_or(definition);
            Some((definition, definition["name"]["$value"].as_str()?))
        })
        .collect();
    if names.iter().any(|(_, name)| *name == requested) {
        return None;
    }
    let mut empty = names.into_iter().filter(|(definition, name)| {
        requested
            .strip_prefix(*name)
            .is_some_and(|suffix| suffix.starts_with('_'))
            && definition
                .as_object()
                .is_some_and(|fields| fields.keys().all(|key| key == "$type" || key == "name"))
    });
    let (_, name) = empty.next()?;
    if empty.next().is_some() {
        return None;
    }
    Some(name.to_owned())
}

pub(crate) fn resolve_all(c: &mut Catalogue, source: &GameSource) {
    let mut groups = BTreeMap::<u64, Vec<String>>::new();
    for (id, selection) in &c.selections {
        if selection.resolution != "pending" {
            continue;
        }
        groups
            .entry(selection.resource.native_hash())
            .or_default()
            .push(id.clone());
    }
    let total: usize = groups.values().map(Vec::len).sum();
    let mut completed = 0_usize;
    let mut meshes = BTreeMap::<(u64, String), Result<MeshAppearanceBinding, String>>::new();
    let overrides = resource_overrides();
    eprintln!("Resolving bindings for {total} selections...");
    for ids in groups.values() {
        // Reuse native documents across a root's appearances, then release them.
        // Retain only compact mesh binding results across roots.
        let mut reader = source.resources();
        for id in ids {
            if c.selections[id].context["prop_kind"] == "mesh"
                || c.selections[id].context["geometry_kind"] == "mesh"
            {
                resolve_mesh(c, id, &mut reader);
                completed += 1;
                continue;
            }
            let selection = &c.selections[id];
            let linked_empty = linked_empty_appearance(c, selection, &mut reader);
            let effective_appearance = linked_empty.as_deref().or(selection.appearance.as_deref());
            let mut used_overrides = BTreeMap::new();
            if let Ok(document) = reader.document(&selection.resource) {
                collect_overrides(&document, &overrides, &mut used_overrides);
            }
            let result = reader
                .assembly_with_resource_overrides(
                    &selection.resource,
                    effective_appearance,
                    &Default::default(),
                    &overrides,
                )
                .map_err(|e| e.to_string());
            let result =
                result.map(|assembly| {
                    let mut materials = Vec::new();
                    let mut failures = Vec::new();
                    for component in &assembly.components {
                        collect_overrides(&component.template, &overrides, &mut used_overrides);
                        let Some(mesh) = component.mesh.as_ref().filter(|_| {
                            component.enabled
                                && component.chunk_mask != 0
                                && !component.is_collapsed()
                                && component.kind != "entAppearanceProxyMeshComponent"
                        }) else {
                            continue;
                        };
                        let binding = meshes
                            .entry((
                                mesh.native_hash(),
                                component
                                    .mesh_appearance_requested
                                    .clone()
                                    .unwrap_or_else(|| "default".into()),
                            ))
                            .or_insert_with(|| {
                                reader
                                    .mesh_appearance_binding(
                                        mesh,
                                        component
                                            .mesh_appearance_requested
                                            .as_deref()
                                            .unwrap_or("default"),
                                    )
                                    .map_err(|e| e.to_string())
                            });
                        match binding {
                            Ok(binding) => materials
                                .push(json!({"component":component.name,"binding":binding})),
                            Err(error) => failures
                                .push(format!("component {} ({mesh}): {error}", component.name)),
                        }
                    }
                    (assembly, materials, failures)
                });
            apply(c, id, result);
            let selection = c.selections.get_mut(id).expect("discovered selection");
            if !used_overrides.is_empty() {
                selection.context["resource_overrides"] = json!(used_overrides);
            }
            if let Some(effective) = linked_empty {
                selection.context["assembly_appearance"] = json!(effective);
                selection.context["native_appearance_resolution"] = json!({
                    "requested":selection.appearance,"effective":effective,
                    "reason":"Linked skin-color choice; the native genital overlay is empty and body skin supplies the appearance"
                });
            }
            completed += 1;
            if completed.is_multiple_of(1000) || completed == total {
                eprintln!("Processed bindings: {completed}/{total}");
            }
        }
    }
    c.update_coverage();
    eprintln!(
        "Binding resolution complete: {} resolved; {} blocked",
        c.coverage["resolved_selections"], c.coverage["blocked_selections"]
    );
}

fn resolve_mesh(c: &mut Catalogue, id: &str, reader: &mut ghostline_red::assets::AssetReader<'_>) {
    let selected = c.selections.get_mut(id).expect("discovered mesh selection");
    match reader.mesh_appearance_binding(
        &selected.resource,
        selected.appearance.as_deref().unwrap_or("default"),
    ) {
        Ok(binding) => {
            selected.preview_state = if binding.shadow_only || binding.non_visual {
                "not_applicable"
            } else {
                "pending"
            }
            .into();
            selected.resolution = "resolved".into();
            selected.context["material_bindings"] = json!([{"component":null,"binding":binding}]);
        }
        Err(error) => {
            selected.resolution = "blocked".into();
            selected.preview_state = "blocked".into();
            selected.diagnostics.push(format!("resolution: {error}"));
        }
    }
}

fn apply(
    c: &mut Catalogue,
    id: &str,
    result: Result<(Assembly, Vec<serde_json::Value>, Vec<String>), String>,
) {
    let selection = c.selections.get_mut(id).expect("discovered selection");
    selection
        .diagnostics
        .retain(|d| !d.starts_with("resolution: "));
    match result {
        Ok((assembly, materials, failures)) => {
            let visible = materials
                .iter()
                .any(|material| material["binding"]["shadow_only"] != true);
            selection.components=assembly.components.iter().map(|v|json!({
                "name":v.name,"native_class":v.kind,"mesh":v.mesh,"mesh_appearance":v.mesh_appearance,"mesh_appearance_requested":v.mesh_appearance_requested,"chunk_mask":v.chunk_mask.to_string(),
                "enabled":v.enabled,"local_transform":v.local.to_cols_array(),"rig":v.rig,
                "parent":{"component":v.parent.component,"slot":v.parent.slot},"control":{"component":v.control.component,"slot":v.control.slot},
                "slots":v.slots.iter().map(|s|json!({"name":s.name,"bone":s.bone,"translation":s.transform.translation,"rotation":s.transform.rotation,"scale":s.transform.scale})).collect::<Vec<_>>()
            })).collect();
            selection.context["material_bindings"] = json!(materials);
            selection.resolution = if failures.is_empty() {
                "resolved"
            } else {
                "blocked"
            }
            .into();
            selection.preview_state = if failures.is_empty() {
                if visible { "pending" } else { "not_applicable" }
            } else {
                "blocked"
            }
            .into();
            if !visible && selection.resolution == "resolved" {
                selection.context["preview_omission"] =
                    json!("Native binding resolution found no enabled visible mesh components");
            }
            selection
                .diagnostics
                .extend(failures.into_iter().map(|e| format!("resolution: {e}")));
            let assets: Vec<_> = assembly
                .components
                .iter()
                .filter_map(|v| v.mesh.clone().map(|key| (key, "mesh")))
                .chain(assembly.rigs.keys().cloned().map(|key| (key, "rig")))
                .collect();
            for (key, role) in assets {
                add_asset(c, key, role);
            }
        }
        Err(error) => {
            selection.components.clear();
            selection.resolution = "blocked".into();
            selection.preview_state = "blocked".into();
            selection.diagnostics.push(format!("resolution: {error}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ghostline_red::assets::ResourceKey;

    #[test]
    fn binding_failures_preserve_partial_details_and_do_not_leave_pending_selections() {
        let mut c:Catalogue=serde_json::from_value(json!({"format":"ghostline-items","version":1,"sources":{},"items":[],"assets":{},"controls":[],"choices":[],"selections":{},"customization_metadata":{},"coverage":{},"diagnostics":[]})).unwrap();
        let resource = ResourceKey::parse("base/test.app");
        let ids: Vec<_> = ["good", "partial", "missing"]
            .into_iter()
            .map(|appearance| {
                crate::build::add_selection(
                    &mut c,
                    resource.clone(),
                    Some(appearance.into()),
                    json!({}),
                )
            })
            .collect();
        let assembly = || Assembly {
            source: resource.clone(),
            appearance: String::new(),
            components: Vec::new(),
            rigs: Default::default(),
        };
        apply(
            &mut c,
            &ids[0],
            Ok((assembly(), vec![json!({"component":"head"})], Vec::new())),
        );
        apply(
            &mut c,
            &ids[1],
            Ok((
                assembly(),
                vec![json!({"component":"known"})],
                vec!["missing appearance".into()],
            )),
        );
        apply(&mut c, &ids[2], Err("missing resource".into()));
        c.update_coverage();
        assert_eq!(
            c.coverage["resolution_states"],
            json!({"blocked":2,"resolved":1})
        );
        assert_eq!(c.selections[&ids[0]].preview_state, "pending");
        assert_eq!(
            c.selections[&ids[1]].context["material_bindings"][0]["component"],
            "known"
        );
        assert_eq!(
            c.selections[&ids[2]].diagnostics,
            vec!["resolution: missing resource"]
        );
        let root = tempfile::tempdir().unwrap();
        crate::publish::initialize(root.path()).unwrap();
        crate::publish::write(&mut c, root.path(), "").unwrap();
        let report: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.path().join("data/binding-report.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(report["failures"].as_array().unwrap().len(), 2);
        crate::publish::verify(root.path()).unwrap();
    }
}
