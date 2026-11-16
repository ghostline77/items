//! Catalogue discovery from compiled item tables, factory registrations and controls.

use crate::model::*;
use anyhow::Result;
use ghostline_red::{
    assets::{AssetReader, ResourceKey},
    schema::RedSchema,
    tweakdb::{self, FlatValue, RecordTypes, TweakDb, TweakId, TweakNames},
};
use ghostline_renderer::GameSource;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

/// Build inputs; the selected compiled table is a complete cache, not an overlay.
#[derive(Debug)]
pub struct BuildInputs<'a> {
    /// Installed game containing unmodified base/EP1 archives.
    pub game: &'a Path,
    /// Reflected native type schema.
    pub schema: Option<&'a Path>,
    /// Exact compiled TweakDB cache selected for this release.
    pub tweakdb: &'a Path,
    /// Optional native length-prefixed/KARK name dictionary.
    pub tweak_names: Option<&'a Path>,
    /// Localization language used for human labels.
    pub language: &'a str,
    /// Declared game build, recorded with the selected input names.
    pub game_version: &'a str,
}

/// Builds native metadata and resolves all discovered bindings without rendering.
///
/// # Errors
/// Returns an error if required tables, schema or archive indices cannot be read.
pub fn build(input: &BuildInputs<'_>) -> Result<Catalogue> {
    let schema = RedSchema::load(Some(input.game), input.schema)?;
    let types = RecordTypes::new(&schema);
    eprintln!("Reading compiled item properties...");
    let bytes = fs::read(input.tweakdb)?;
    let db = TweakDb::from_bytes(&bytes)?;
    let mut sources = BTreeMap::from([
        ("game_version".into(), input.game_version.into()),
        ("language".into(), input.language.into()),
        (
            "tweakdb_file".into(),
            input
                .tweakdb
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
        ),
    ]);
    drop(bytes);
    let names = if let Some(path) = input.tweak_names {
        let bytes = fs::read(path)?;
        sources.insert(
            "tweak_names_file".into(),
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        );
        eprintln!("Resolving native name dictionary...");
        TweakNames::resolve(&db, tweakdb::read_names(&bytes)?)
    } else {
        TweakNames::default()
    };
    let source = GameSource::open(input.game, input.schema)?;
    let mut reader = source.resources();
    let mut catalogue = Catalogue {
        format: "ghostline-items".into(),
        version: 1,
        sources,
        items: Vec::new(),
        assets: BTreeMap::new(),
        props: Vec::new(),
        controls: Vec::new(),
        choices: Vec::new(),
        selections: BTreeMap::new(),
        customization_metadata: BTreeMap::new(),
        coverage: json!({}),
        diagnostics: Vec::new(),
    };
    let factories = factories(&mut reader, &mut catalogue.diagnostics);
    let localized = localization(&mut reader, input.language, &mut catalogue.diagnostics);
    let mut appearances = BTreeMap::new();
    for (&id, &class) in &db.records {
        if !types.is_a(class, "gamedataItem_Record") {
            continue;
        }
        let native_name = names.records.get(&id).cloned();
        let mut fields: BTreeSet<String> = FALLBACK_FIELDS
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        if let Some(props) = names.properties.get(&id) {
            fields.extend(props.keys().cloned());
        }
        let properties: BTreeMap<String, FlatValue> = fields
            .into_iter()
            .filter_map(|p| db.property(id, &p).map(|v| (p, v.clone())))
            .collect();
        let label = properties
            .get("displayName")
            .and_then(|v| value_text(&v.value))
            .and_then(|k| localized.get(k))
            .and_then(local_text)
            .map(str::to_owned)
            .or_else(|| {
                properties
                    .get("localizedName")
                    .and_then(|v| value_text(&v.value))
                    .and_then(|k| localized.get(k))
                    .and_then(local_text)
                    .map(str::to_owned)
            })
            .or_else(|| native_name.clone())
            .unwrap_or_else(|| format!("Item {id}"));
        let category = linked_name(&db, &names, id, "itemCategory");
        let item_type = linked_name(&db, &names, id, "itemType");
        let localization = ["displayName", "localizedName", "localizedDescription"]
            .into_iter()
            .filter_map(|field| {
                let key = properties.get(field)?.value.as_str()?;
                localized
                    .get(key)
                    .map(|entry| (field.to_owned(), entry.clone()))
            })
            .collect();
        let mut item = Item {
            id: format!("item:{id}"),
            native_id: id,
            native_name,
            label,
            native_class: types.name(class).unwrap_or("unknown").into(),
            category,
            item_type,
            properties,
            localization,
            selections: Vec::new(),
            diagnostics: Vec::new(),
        };
        discover_item(
            &mut item,
            &factories,
            &mut reader,
            &mut appearances,
            &mut catalogue,
        );
        catalogue.items.push(item);
    }
    eprintln!(
        "Discovered {} item records; reading customization roots...",
        catalogue.items.len()
    );
    for (prefix, stem) in [
        ("base", "female_cco"),
        ("base", "male_cco"),
        ("ep1", "female_cco_ep1"),
        ("ep1", "male_cco_ep1"),
    ] {
        let path =
            format!("{prefix}/gameplay/gui/fullscreen/main_menu/{stem}.inkcharcustomization");
        let key = ResourceKey::parse(&path);
        match reader.customization(&key) {
            Ok(custom) => {
                // Locators are scoped to the complete decoded native root, not menu indices alone.
                catalogue
                    .customization_metadata
                    .insert(path.clone(), custom.metadata);
                for native in custom.controls {
                    let id = format!(
                        "{path}#{}CustomizationOptions/{}",
                        native.domain, native.source_index
                    );
                    collect_assets(&native.data, "customization", &mut catalogue);
                    let mut choices = Vec::new();
                    for kind in ["options", "definitions", "morphNames"] {
                        if let Some(entries) = native.data.get(kind).and_then(Value::as_array) {
                            for (index, data) in entries.iter().enumerate() {
                                let choice_id = format!("{id}/{kind}/{index}");
                                let label = data
                                    .get("localizedName")
                                    .and_then(value_text)
                                    .map(|s| localized_label(s, &localized))
                                    .or_else(|| {
                                        data.get("name").and_then(value_text).map(str::to_owned)
                                    })
                                    .unwrap_or_else(|| {
                                        format!(
                                            "{} / {index}",
                                            native.name.as_deref().unwrap_or(&native.native_class)
                                        )
                                    });
                                let target = native
                                    .data
                                    .get("resource")
                                    .map(ResourceKey::from_value)
                                    .transpose()?
                                    .flatten();
                                let selection = if kind == "definitions" {
                                    target.and_then(|resource|data.get("name").and_then(value_text).map(|appearance|add_selection(&mut catalogue,resource,Some(appearance.into()),json!({"kind":"isolated_resource","compatibility":"unknown","reference_assembly_required_for_combined_preview":true}))))
                                } else {
                                    None
                                };
                                choices.push(choice_id.clone());
                                catalogue.choices.push(Choice {
                                    id: choice_id,
                                    control: id.clone(),
                                    kind: kind.into(),
                                    source_index: index,
                                    label,
                                    data: data.clone(),
                                    selection,
                                });
                            }
                        }
                    }
                    catalogue.controls.push(Control {
                        id,
                        source: key.clone(),
                        native,
                        choices,
                    });
                }
            }
            Err(e) => catalogue
                .diagnostics
                .push(format!("customization {path}: {e}")),
        }
    }
    catalogue.coverage = json!({"native_records":db.records.len(),"native_flats":db.flats.len(),
        "native_queries":db.queries.len(),"unresolved_record_classes":db.records.values().filter(|h|types.name(**h).is_none()).count(),
        "named_records":names.records.len(),"named_item_records":catalogue.items.iter().filter(|i|i.native_name.is_some()).count(),
        "known_named_flats":names.properties.values().map(BTreeMap::len).sum::<usize>(),
        "factory_registrations":factories.len(),"localization_entries":localized.len(),
        "scope":"Selected complete compiled cache; unmodified base/EP1 archives; authored customization choices and referenced assets. No runtime mod substitutions or inferred spawnability."});
    drop(reader);
    drop(db);
    drop(names);
    drop(appearances);
    drop(factories);
    drop(localized);
    crate::bindings::resolve_all(&mut catalogue, &source);
    Ok(catalogue)
}

fn linked_name(db: &TweakDb, names: &TweakNames, id: TweakId, field: &str) -> Option<String> {
    let v = db
        .property(id, field)?
        .value
        .as_str()?
        .parse()
        .ok()
        .map(TweakId)?;
    if v.0 == 0 {
        return None;
    }
    db.property(v, "name")
        .and_then(|p| p.value.as_str())
        .map(str::to_owned)
        .or_else(|| names.records.get(&v).cloned())
        .or_else(|| Some(v.to_string()))
}

fn factories(
    reader: &mut AssetReader<'_>,
    diagnostics: &mut Vec<String>,
) -> BTreeMap<String, Vec<ResourceKey>> {
    let mut out = BTreeMap::<String, Vec<ResourceKey>>::new();
    let mut paths = Vec::new();
    for registry in ["base/gameplay/factories.csv", "ep1/gameplay/factories.csv"] {
        match reader.table(&ResourceKey::parse(registry)) {
            Ok(list) => paths.extend(
                list.rows
                    .into_iter()
                    .filter_map(|row| row.into_iter().next()),
            ),
            Err(e) => diagnostics.push(format!("factory registry {registry}: {e}")),
        }
    }
    for path in &paths {
        let table = match reader.table(&ResourceKey::parse(path)) {
            Ok(t) => t,
            Err(e) => {
                diagnostics.push(format!("factory {path}: {e}"));
                continue;
            }
        };
        let (Some(name), Some(resource)) = (
            table.headers.iter().position(|x| x == "name"),
            table.headers.iter().position(|x| x == "path"),
        ) else {
            continue;
        };
        for row in table.rows {
            if let (Some(name), Some(path)) = (row.get(name), row.get(resource)) {
                if name.is_empty() || path.is_empty() {
                    continue;
                }
                let key = ResourceKey::parse(path);
                let entries = out.entry(name.clone()).or_default();
                if !entries.contains(&key) {
                    entries.push(key);
                }
            }
        }
    }
    out
}

fn localization(
    reader: &mut AssetReader<'_>,
    language: &str,
    diagnostics: &mut Vec<String>,
) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for prefix in ["base", "ep1"] {
        let path = format!("{prefix}/localization/{language}/onscreens/onscreens_final.json");
        match reader.document(&ResourceKey::parse(&path)) {
            Ok(document) => walk(&document, &mut |v| {
                if let Some(k) = v.get("primaryKey").and_then(value_text)
                    && local_text(v).is_some()
                {
                    out.insert(k.into(), v.clone());
                    if let Some(secondary) = v
                        .get("secondaryKey")
                        .and_then(value_text)
                        .filter(|s| !s.is_empty())
                    {
                        out.insert(secondary.into(), v.clone());
                    }
                }
            }),
            Err(e) => diagnostics.push(format!("localization {path}: {e}")),
        }
    }
    out
}
fn local_text(value: &Value) -> Option<&str> {
    value
        .get("femaleVariant")
        .and_then(value_text)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            value
                .get("maleVariant")
                .and_then(value_text)
                .filter(|s| !s.is_empty())
        })
}

fn discover_item(
    item: &mut Item,
    factories: &BTreeMap<String, Vec<ResourceKey>>,
    reader: &mut AssetReader<'_>,
    cache: &mut BTreeMap<String, Result<Vec<ghostline_red::assets::Appearance>, String>>,
    catalogue: &mut Catalogue,
) {
    let entity = item
        .properties
        .get("entityName")
        .and_then(|v| v.value.as_str())
        .unwrap_or_default();
    let override_name = item
        .properties
        .get("appearanceResourceName")
        .and_then(|v| v.value.as_str())
        .filter(|v| !v.is_empty() && *v != "None");
    let appearance = item
        .properties
        .get("appearanceName")
        .and_then(|v| v.value.as_str())
        .filter(|v| !v.is_empty() && *v != "None");
    let registration = override_name.unwrap_or(entity);
    if registration.is_empty() || registration == "None" {
        item.diagnostics
            .push("No registered visual root on this compiled record".into());
        return;
    }
    let Some(resources) = factories.get(registration) else {
        item.diagnostics
            .push(format!("Unresolved factory registration {registration}"));
        return;
    };
    for resource in resources {
        add_asset(catalogue, resource.clone(), "item_root");
        let entries = cache
            .entry(resource.to_string())
            .or_insert_with(|| reader.appearances(resource).map_err(|e| e.to_string()));
        match entries {
            Ok(entries) => {
                let matches: Vec<_> = entries
                    .iter()
                    .filter(|e| {
                        if appearance.is_none() && resource.to_string().ends_with(".app") {
                            return true;
                        }
                        appearance.is_some_and(|a| {
                            e.name == a
                                || e.name
                                    .strip_prefix(a)
                                    .is_some_and(|suffix| suffix.starts_with('&'))
                        })
                    })
                    .collect();
                if entries.is_empty() {
                    item.selections.push(add_selection(
                        catalogue,
                        resource.clone(),
                        None,
                        json!({"kind":"isolated_resource","compatibility":"unknown"}),
                    ));
                } else if matches.is_empty() {
                    item.diagnostics.push(format!(
                        "No exact authored match for {appearance:?} in {resource}"
                    ));
                } else {
                    if appearance.is_none() {
                        item.diagnostics.push("Resource-only binding: authored variants listed; no default record appearance asserted".into());
                    }
                    for entry in matches {
                        if let Some(target) = &entry.resource {
                            add_asset(catalogue, target.clone(), "appearance_resource");
                        }
                        item.selections.push(add_selection(
                            catalogue,
                            resource.clone(),
                            Some(entry.name.clone()),
                            json!({"kind":"isolated_resource","compatibility":"unknown"}),
                        ));
                    }
                }
            }
            Err(e) => item
                .diagnostics
                .push(format!("appearance discovery {resource}: {e}")),
        }
    }
    item.selections.sort();
    item.selections.dedup();
}

/// Registers an exact native resource/appearance selection.
pub fn add_selection(
    c: &mut Catalogue,
    resource: ResourceKey,
    appearance: Option<String>,
    context: Value,
) -> String {
    add_asset(c, resource.clone(), "selection_root");
    let id = selection_key(&resource, appearance.as_deref());
    c.selections.entry(id.clone()).or_insert_with(|| Selection {
        id: id.clone(),
        resource,
        appearance,
        context,
        components: Vec::new(),
        resolution: "pending".into(),
        preview_state: "pending".into(),
        diagnostics: Vec::new(),
        media: Vec::new(),
    });
    id
}

/// Adds a first-class native resource, merging observed roles without duplication.
pub fn add_asset(c: &mut Catalogue, resource: ResourceKey, role: &str) {
    let id = format!("asset:{}", resource.native_hash());
    let a = c.assets.entry(id.clone()).or_insert_with(|| VisualAsset {
        selections: Vec::new(),
        id,
        resource,
        roles: Vec::new(),
    });
    if !a.roles.iter().any(|r| r == role) {
        a.roles.push(role.into());
        a.roles.sort();
    }
}
fn collect_assets(value: &Value, role: &str, c: &mut Catalogue) {
    walk(value, &mut |v| {
        if v.get("DepotPath").is_some()
            && let Ok(Some(resource)) = ResourceKey::from_value(v)
        {
            add_asset(c, resource, role);
        }
    });
}
fn walk(value: &Value, visit: &mut impl FnMut(&Value)) {
    visit(value);
    match value {
        Value::Object(o) => {
            for v in o.values() {
                walk(v, visit);
            }
        }
        Value::Array(a) => {
            for v in a {
                walk(v, visit);
            }
        }
        _ => {}
    }
}
fn value_text(v: &Value) -> Option<&str> {
    v.get("$value").unwrap_or(v).as_str()
}
fn localized_label(label: &str, localized: &BTreeMap<String, Value>) -> String {
    localized
        .get(label.strip_prefix("LocKey#").unwrap_or(label))
        .and_then(local_text)
        .map(str::to_owned)
        .unwrap_or_else(|| label.into())
}

// Direct lookup remains available without a name dictionary. A verified dictionary
// adds every matching named property, including properties absent from this list.
const FALLBACK_FIELDS: &str = "displayName localizedName localizedDescription friendlyName entityName appearanceName appearanceResourceName itemCategory itemType equipArea equipAreas placementSlots attachmentSlots requiredSlots tags visualTags icon iconPath quality mass isGarment isPart isCustomizable deprecated counterpart equivalent nextUpgradeItem sideUpgradeItem variants parts slotPartList slotPartListPreset blueprint OnEquip OnAttach OnLooted onEquipStats buyPrice sellPrice statModifiers statModifierGroups appearanceSuffixes appearanceSuffixesOwnerOverride useNewSpawnMethod hairSkinnedMeshComponents garmentOffset";

/// Readable native resource/appearance identity.
pub fn selection_key(resource: &ResourceKey, appearance: Option<&str>) -> String {
    format!("{}#{}", resource, appearance.unwrap_or_default())
}
