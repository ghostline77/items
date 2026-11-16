//! Resource-based garment discovery, independent of player equipment records.
use crate::{bindings, build, model::Catalogue};
use anyhow::{Context, Result};
use ghostline_red::assets::{AssetReader, ResourceKey};
use ghostline_renderer::GameSource;
use serde_json::json;
use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    time::{Duration, Instant},
};

/// Discovers base/EP1 garments and clothing referenced by native character
/// appearances, retaining exact mesh appearances and ownership evidence.
/// Names identify candidates; only installed resources are registered.
///
/// # Errors
/// Returns an error if the name dictionary cannot be read or decoded. Resource
/// failures remain explicit catalogue diagnostics or blocked selections.
pub fn extend(c: &mut Catalogue, source: &GameSource, names_file: &Path) -> Result<()> {
    let bytes = fs::read(names_file).context("read clothing resource names")?;
    let paths: BTreeSet<_> = AssetReader::resource_names(&bytes)?
        .into_iter()
        .filter(|path| {
            let character =
                path.starts_with("base\\characters\\") || path.starts_with("ep1\\characters\\");
            let garment = path.starts_with("base\\characters\\garment\\")
                || path.starts_with("ep1\\characters\\garment\\");
            character
                && (path.ends_with(".ent")
                    || path.ends_with(".app")
                    || garment && (path.ends_with(".mesh") || path.ends_with(".morphtarget")))
        })
        .collect();
    let mut registered = BTreeSet::new();
    let start = Instant::now();
    let mut reported = Instant::now();
    eprintln!(
        "Scanning {} clothing/character resource candidates...",
        paths.len()
    );
    for (i, path) in paths.iter().enumerate() {
        if reported.elapsed() >= Duration::from_secs(5) {
            eprintln!(
                "Clothing discovery: {i}/{} candidates; {} garment resources; {:.1}s; {path}",
                paths.len(),
                registered.len(),
                start.elapsed().as_secs_f64()
            );
            reported = Instant::now();
        }
        if !(path.starts_with("base\\characters\\") || path.starts_with("ep1\\characters\\")) {
            continue;
        }
        let garment = path.starts_with("base\\characters\\garment\\")
            || path.starts_with("ep1\\characters\\garment\\");
        let mesh = path.ends_with(".mesh") || path.ends_with(".morphtarget");
        let assembly = path.ends_with(".ent") || path.ends_with(".app");
        if !(garment && mesh || assembly) {
            continue;
        }
        let mut reader = source.resources();
        let resource = ResourceKey::parse(path);
        if !reader.resource_exists(&resource) {
            continue;
        }
        if garment {
            register(c, &mut reader, resource.clone(), mesh, &mut registered);
        }
        if !assembly {
            continue;
        }
        let appearances = match reader.appearances(&resource) {
            Ok(entries) if entries.is_empty() => vec![None],
            Ok(entries) => entries.into_iter().map(|a| Some(a.name)).collect(),
            Err(error) => {
                c.diagnostics
                    .push(format!("clothing discovery: {resource}: {error}"));
                continue;
            }
        };
        for appearance in appearances {
            match reader.assembly_manifest(&resource, appearance.as_deref()) {
                Ok(native) => {
                    for component in native.components {
                        if !component.kind.contains("Garment") {
                            continue;
                        }
                        let Some(mesh) = component.mesh else { continue };
                        // Decode each garment's appearance list once across all
                        // NPC owners. Discovery does not need rigs or other meshes.
                        register(
                            c,
                            &mut source.resources(),
                            mesh.clone(),
                            true,
                            &mut registered,
                        );
                        let asset = c
                            .assets
                            .get_mut(&format!("asset:{}", mesh.native_hash()))
                            .expect("registered clothing mesh");
                        let requested = component
                            .mesh_appearance_requested
                            .as_deref()
                            .unwrap_or("default");
                        let exact = build::selection_key(&mesh, Some(requested));
                        let id = if asset.selections.contains(&exact) {
                            Some(exact)
                        } else {
                            asset.selections.first().cloned()
                        };
                        if let Some(id) = id {
                            let selection = c.selections.get_mut(&id).expect("registered garment");
                            let owners = selection
                                .context
                                .as_object_mut()
                                .expect("selection context")
                                .entry("clothing_owners")
                                .or_insert_with(|| json!([]));
                            let owner = json!({"resource":resource,"appearance":appearance,
                                "component":component.name,"mesh_appearance":component.mesh_appearance_requested});
                            if let Some(owners) = owners.as_array_mut()
                                && !owners.contains(&owner)
                            {
                                owners.push(owner);
                            }
                        }
                    }
                }
                Err(error) => c.diagnostics.push(format!(
                    "clothing discovery: {resource}#{}: {error}",
                    appearance.as_deref().unwrap_or("default")
                )),
            }
        }
    }
    c.sources
        .insert("clothing_source_identity".into(), source.content_identity());
    c.coverage["clothing_resources"] = json!(registered.len());
    eprintln!(
        "Discovered {} native clothing resources; resolving selections...",
        registered.len()
    );
    bindings::resolve_all(c, source);
    Ok(())
}

fn register(
    c: &mut Catalogue,
    reader: &mut AssetReader<'_>,
    resource: ResourceKey,
    mesh: bool,
    registered: &mut BTreeSet<u64>,
) {
    if !registered.insert(resource.native_hash()) {
        return;
    }
    build::add_asset(c, resource.clone(), "clothing");
    let appearances = if mesh {
        reader
            .mesh_appearance_names(&resource)
            .map(|v| v.into_iter().map(Some).collect::<Vec<_>>())
    } else {
        reader.appearances(&resource).map(|v| {
            if v.is_empty() {
                vec![None]
            } else {
                v.into_iter().map(|a| Some(a.name)).collect()
            }
        })
    };
    let (appearances, error) = match appearances {
        Ok(v) => (v, None),
        Err(e) => (vec![None], Some(e.to_string())),
    };
    for appearance in appearances {
        let id = build::add_selection(
            c,
            resource.clone(),
            appearance,
            json!({"kind":"clothing","clothing":true,"geometry_kind":if mesh {"mesh"} else {"assembly"}}),
        );
        let selection = c.selections.get_mut(&id).expect("registered garment");
        selection.context["clothing"] = json!(true);
        if let Some(error) = &error {
            selection.resolution = "blocked".into();
            selection.preview_state = "blocked".into();
            selection
                .diagnostics
                .push(format!("resolution: clothing discovery: {error}"));
        }
        let asset = c
            .assets
            .get_mut(&format!("asset:{}", resource.native_hash()))
            .expect("registered asset");
        if !asset.selections.contains(&id) {
            asset.selections.push(id);
        }
    }
}
