//! Native world-prop discovery, complete assemblies and individual mesh relations.
use crate::{
    bindings, build,
    model::{Catalogue, Prop},
};
use anyhow::{Context, Result};
use ghostline_red::assets::{AssetReader, ResourceKey};
use ghostline_renderer::{AssemblyOptions, GameSource, SceneAssembly};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    sync::Arc,
};

/// Native name hints and optional depot prefixes defining a prop inventory.
pub struct PropInputs<'a> {
    /// UTF-8 newline-separated depot paths, optionally KARK compressed.
    pub resource_names: &'a Path,
    /// Depot prefixes; empty selects the standard world/item prop roots.
    pub prefixes: &'a [String],
}

/// Creates a prop-only catalogue without requiring compiled gameplay tables.
pub fn empty_catalogue(game_version: &str, language: &str) -> Catalogue {
    Catalogue {
        format: "ghostline-items".into(),
        version: 1,
        sources: BTreeMap::from([
            ("game_version".into(), game_version.into()),
            ("language".into(), language.into()),
        ]),
        items: Vec::new(),
        assets: BTreeMap::new(),
        props: Vec::new(),
        controls: Vec::new(),
        choices: Vec::new(),
        selections: BTreeMap::new(),
        customization_metadata: BTreeMap::new(),
        coverage: json!({}),
        diagnostics: Vec::new(),
    }
}

/// Adds native prop meshes and entity assemblies, resolving every binding and
/// rigid part placement before publication. Names only identify candidates;
/// installed native resources establish geometry, membership and placements.
///
/// # Errors
/// Returns errors for unreadable name inputs. Per-resource errors are retained in
/// the catalogue and binding report rather than aborting discovery.
pub fn extend(c: &mut Catalogue, source: &GameSource, input: &PropInputs<'_>) -> Result<()> {
    let bytes = fs::read(input.resource_names).context("read prop resource names")?;
    let names = AssetReader::resource_names(&bytes)?;
    let prefixes: Vec<_> = if input.prefixes.is_empty() {
        [
            "base/environment/",
            "ep1/environment/",
            "base/items/",
            "ep1/items/",
            "base/gameplay/devices/",
            "ep1/gameplay/devices/",
            "base/meshes/",
            "ep1/meshes/",
            "base/props/",
            "ep1/props/",
            "base/prefabs/",
            "ep1/prefabs/",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    } else {
        input.prefixes.to_vec()
    }
    .into_iter()
    .map(|p| p.replace('/', "\\").to_lowercase())
    .collect();
    c.sources.insert(
        "props_names_file".into(),
        input
            .resource_names
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
    );
    c.sources
        .insert("props_prefixes".into(), serde_json::to_string(&prefixes)?);
    c.sources
        .insert("props_source_identity".into(), source.content_identity());
    let mut index: BTreeMap<_, _> = c
        .props
        .iter()
        .enumerate()
        .map(|(i, p)| (p.resource.native_hash(), i))
        .collect();
    let paths: BTreeSet<_> = names
        .into_iter()
        .filter(|path| {
            let path = path.to_lowercase();
            (path.ends_with(".mesh") || path.ends_with(".morphtarget") || path.ends_with(".ent"))
                && prefixes.iter().any(|prefix| path.starts_with(prefix))
        })
        .collect();
    c.coverage["prop_name_candidates"] = json!(paths.len());
    for path in paths {
        // Release reflected mesh/entity documents after each discovery root;
        // a full world catalogue must not retain every decoded vertex buffer.
        let mut reader = source.resources();
        let key = ResourceKey::parse(&path);
        if reader.resource_exists(&key) {
            register(c, &mut reader, key, &mut index);
        }
    }
    eprintln!(
        "Discovered {} native prop resources; resolving selections...",
        c.props.len()
    );
    resolve_assemblies(c, source)
}

/// Resolves pending bindings and evaluates native assembly membership/placements.
/// Already evaluated assemblies retain their published instances during resume.
///
/// # Errors
/// Returns publication metadata errors; native failures stay on their selections.
pub fn resolve_assemblies(c: &mut Catalogue, source: &GameSource) -> Result<()> {
    let mut index: BTreeMap<_, _> = c
        .props
        .iter()
        .enumerate()
        .map(|(i, p)| (p.resource.native_hash(), i))
        .collect();
    bindings::resolve_all(c, source);
    let assemblies: Vec<_> = c
        .props
        .iter()
        .enumerate()
        .filter(|(_, p)| p.kind == "assembly")
        .map(|(i, p)| (i, p.id.clone(), p.selections.clone()))
        .collect();
    let count = assemblies.len();
    let mut clock = std::time::Instant::now();
    for (processed, (owner_index, owner, selections)) in assemblies.into_iter().enumerate() {
        if clock.elapsed() >= std::time::Duration::from_secs(5) {
            eprintln!("Assembly placements: {processed}/{count}; {owner}");
            clock = std::time::Instant::now();
        }
        let mut reader = source.resources();
        let mut parts = BTreeSet::new();
        for id in selections {
            if c.selections[&id].resolution != "resolved"
                || c.selections[&id].preview_state == "not_applicable"
            {
                continue;
            }
            if c.selections[&id].context["prop_components"].is_array() {
                for member in c.selections[&id].context["prop_components"]
                    .as_array()
                    .expect("checked array")
                {
                    if let Some(part) = member["part"].as_str() {
                        parts.insert(part.to_owned());
                    }
                }
                continue;
            }
            let components = c.selections[&id].components.clone();
            let mut instances = Vec::new();
            for component in components.iter().filter(|component| {
                component["enabled"] == true
                    && component["chunk_mask"] != "0"
                    && component["native_class"] != "entAppearanceProxyMeshComponent"
            }) {
                let Ok(mesh) = serde_json::from_value::<ResourceKey>(component["mesh"].clone())
                else {
                    continue;
                };
                let part = register(c, &mut reader, mesh, &mut index);
                let member = &mut c.props[part];
                if !member.assemblies.contains(&owner) {
                    member.assemblies.push(owner.clone());
                }
                parts.insert(member.id.clone());
                let appearance = component["mesh_appearance"].as_str().unwrap_or("default");
                let selection = build::selection_key(&member.resource, Some(appearance));
                instances.push(
                    json!({"component":component["name"],"part":member.id,"selection":selection}),
                );
            }
            let selected = &c.selections[&id];
            let placements =
                (|| -> Result<_> {
                    let mut assembly =
                        reader.assembly(&selected.resource, selected.appearance.as_deref())?;
                    let mut options = AssemblyOptions::standalone(&assembly);
                    match selected.resource.native_hash() {
                        // This decal part declares externalAnim -> anim. Its native
                        // owner supplies the shutter rig and authored gate1 frame.
                        8_285_163_056_628_687_803 => options.binding_contexts.push(
                            ghostline_renderer::AssemblyReference {
                                frame: false,
                                resource: ResourceKey::parse(
                                    "base/gameplay/devices/doors/gates/mall_shutter_w300_h250.ent",
                                ),
                                appearance: Some("shutter_w300_h250_lizzies".into()),
                            },
                        ),
                        // The right light requests the stale Base slot; the actual
                        // headset slot is main, on the native base bone.
                        9_203_605_220_523_941_873 => {
                            options
                                .slot_bone_overrides
                                .insert("Slot5946/Base".into(), "base".into());
                        }
                        _ => {}
                    }
                    source.add_binding_context(&mut assembly, &options)?;
                    options
                        .origin_anchors
                        .extend(AssemblyOptions::standalone(&assembly).origin_anchors);
                    let scene = SceneAssembly::from_native(assembly, &options, |mesh, _| {
                        source
                            .load_geometry(mesh)?
                            .with_neutral_material()
                            .map(Arc::new)
                    })?;
                    let contexts = options.binding_contexts;
                    let slot_overrides = options.slot_bone_overrides;
                    Ok((
                        scene.native_component_placements()?,
                        options.origin_anchors,
                        scene.preview_notes,
                        contexts,
                        slot_overrides,
                    ))
                })();
            let selected = c.selections.get_mut(&id).expect("registered assembly");
            match placements {
                Ok((placements, origins, notes, contexts, slot_overrides)) => {
                    selected.context["binding_contexts"] = json!(contexts);
                    selected.context["slot_bone_overrides"] = json!(slot_overrides);
                    selected.context["standalone_origin_anchors"] = json!(origins);
                    selected.context["placement_notes"] = json!(notes);
                    for instance in &mut instances {
                        if let Some(matrix) = instance["component"]
                            .as_str()
                            .and_then(|name| placements.get(name))
                        {
                            instance["native_world_transform"] = json!(matrix);
                        }
                    }
                    selected.context["prop_components"] = json!(instances);
                    selected.context["placement_space"] = json!(
                        "entity-relative, metres, native RED Z-up, column-major matrices; native rig rest pose"
                    );
                }
                Err(error) => {
                    selected.resolution = "blocked".into();
                    selected.preview_state = "blocked".into();
                    selected
                        .diagnostics
                        .push(format!("resolution: native prop placements: {error:#}"));
                }
            }
        }
        c.props[owner_index].parts = parts.into_iter().collect();
    }
    bindings::resolve_all(c, source);
    c.update_coverage();
    Ok(())
}

fn register(
    c: &mut Catalogue,
    reader: &mut AssetReader<'_>,
    resource: ResourceKey,
    index: &mut BTreeMap<u64, usize>,
) -> usize {
    if let Some(&i) = index.get(&resource.native_hash()) {
        return i;
    }
    let path = resource.to_string();
    let mesh = path.ends_with(".mesh")
        || path.ends_with(".morphtarget")
        || reader.document(&resource).is_ok_and(|doc| {
            matches!(
                doc["Data"]["RootChunk"]["$type"].as_str(),
                Some("CMesh" | "MorphTargetMesh" | "CPhysicsDecorationResource")
            )
        });
    let kind = if mesh { "mesh" } else { "assembly" };
    let id = format!("prop:{}", resource.native_hash());
    let stem = path
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(&path)
        .split('.')
        .next()
        .unwrap_or(&path);
    let stem = stem.split_once("__").map_or(stem, |(_, name)| name);
    let label = stem.replace('_', " ");
    let category = if path.contains("\\furniture\\") {
        "Furniture"
    } else if path.contains("\\electronics\\") {
        "Electronics"
    } else {
        "Props"
    };
    let mut diagnostics = Vec::new();
    let appearances = (|| -> Result<Vec<Option<String>>> {
        if mesh {
            Ok(reader
                .mesh_appearance_names(&resource)?
                .into_iter()
                .map(Some)
                .collect())
        } else {
            let entries = reader.appearances(&resource)?;
            Ok(if entries.is_empty() {
                vec![None]
            } else {
                entries.into_iter().map(|a| Some(a.name)).collect()
            })
        }
    })();
    let appearances = appearances.unwrap_or_else(|error| {
        diagnostics.push(format!("native discovery: {error:#}"));
        vec![None]
    });
    let selections = appearances
        .into_iter()
        .map(|appearance| {
            let selection = build::add_selection(
                c,
                resource.clone(),
                appearance,
                json!({"kind":if mesh {"prop_mesh"} else {"prop_assembly"},"prop":id}),
            );
            // A prop can already be present through a gameplay record; retain its
            // native identity while adding the world-object preview role.
            c.selections
                .get_mut(&selection)
                .expect("registered selection")
                .context["prop"] = json!(id);
            c.selections
                .get_mut(&selection)
                .expect("registered selection")
                .context["prop_kind"] = json!(kind);
            selection
        })
        .collect();
    build::add_asset(
        c,
        resource.clone(),
        if mesh { "prop_mesh" } else { "prop_assembly" },
    );
    let i = c.props.len();
    index.insert(resource.native_hash(), i);
    c.props.push(Prop {
        id,
        label,
        resource,
        kind: kind.into(),
        category: category.into(),
        selections,
        parts: Vec::new(),
        assemblies: Vec::new(),
        diagnostics,
    });
    i
}
