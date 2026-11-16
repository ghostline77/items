//! Versioned published catalogue entities. All native 64-bit identities are strings.

use ghostline_red::{
    assets::{CustomizationControl, ResourceKey},
    tweakdb::{FlatValue, TweakId},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// A native-source catalogue, shared by JSON and browser consumers.
#[derive(Debug, Serialize, Deserialize)]
pub struct Catalogue {
    /// Contract identifier.
    pub format: String,
    /// Contract version.
    pub version: u32,
    /// Selected game version, source filenames and language.
    pub sources: BTreeMap<String, String>,
    /// Exact native item records.
    pub items: Vec<Item>,
    /// First-class resources, including those with no gameplay item record.
    pub assets: BTreeMap<String, VisualAsset>,
    /// World props as native entity assemblies and independently selectable meshes.
    #[serde(default)]
    pub props: Vec<Prop>,
    /// Native customization controls and lossless activation metadata.
    pub controls: Vec<Control>,
    /// Authored native choices, distinct from controls and records.
    pub choices: Vec<Choice>,
    /// Exact selection bindings, including unresolved context requirements.
    pub selections: BTreeMap<String, Selection>,
    /// Per-root group, preset and perspective metadata.
    pub customization_metadata: BTreeMap<String, Value>,
    /// Discovery, resolution and preview accounting.
    pub coverage: Value,
    /// Source-level diagnostics; individual entries also carry diagnostics.
    pub diagnostics: Vec<String>,
}

/// Native gameplay record and its flattened compiled properties.
#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
    /// Catalogue record identity.
    pub id: String,
    /// Native TweakDB identity.
    pub native_id: TweakId,
    /// Verified spelling from an optional name dictionary.
    pub native_name: Option<String>,
    /// Localized display name, falling back to known native identity.
    pub label: String,
    /// Concrete reflected record class.
    pub native_class: String,
    /// Native item category, resolved through its referenced record.
    pub category: Option<String>,
    /// Native item type, resolved through its referenced record.
    pub item_type: Option<String>,
    /// Compiled properties, with typed native values and exact hashes.
    pub properties: BTreeMap<String, FlatValue>,
    /// Source localization entries for names/descriptions, retaining both variants.
    pub localization: BTreeMap<String, Value>,
    /// Exact selection identities found for this record.
    pub selections: Vec<String>,
    /// Unresolved source links or visual constraints.
    pub diagnostics: Vec<String>,
}

/// Reusable native resource independent of inventory ownership.
#[derive(Debug, Serialize, Deserialize)]
pub struct VisualAsset {
    /// Exact independently selectable appearances, including NPC-only garments.
    #[serde(default)]
    pub selections: Vec<String>,
    /// Catalogue asset identity derived from native depot hash.
    pub id: String,
    /// Resource path or native hash.
    pub resource: ResourceKey,
    /// Observed catalogue roles; no compatibility is implied.
    pub roles: Vec<String>,
}

/// Named native control with a source-snapshot-scoped catalogue identity.
#[derive(Debug, Serialize, Deserialize)]
pub struct Control {
    /// Catalogue identity, including owner resource/domain/source locator.
    pub id: String,
    /// Native owner resource.
    pub source: ResourceKey,
    /// Full typed native control.
    pub native: CustomizationControl,
    /// Child choice IDs in native order.
    pub choices: Vec<String>,
}

/// A single authored switcher option, appearance definition or morph entry.
#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    /// Source-scoped catalogue choice identity.
    pub id: String,
    /// Owning control ID.
    pub control: String,
    /// Native collection kind.
    pub kind: String,
    /// Position in the native collection, not a cross-release ID.
    pub source_index: usize,
    /// Localized label or native label/identity fallback.
    pub label: String,
    /// Lossless decoded choice data, including omitted-index semantics.
    pub data: Value,
    /// Candidate selection, when this choice directly describes an appearance.
    pub selection: Option<String>,
}

/// Exact root/appearance binding plus explicit preparation and preview state.
#[derive(Debug, Serialize, Deserialize)]
pub struct Selection {
    /// Catalogue identity, derived from the native root and appearance.
    pub id: String,
    /// Native resource to assemble.
    pub resource: ResourceKey,
    /// Exact authored entity/appearance name.
    pub appearance: Option<String>,
    /// Source context; never inferred body-fit compatibility.
    pub context: Value,
    /// Native component data after successful resolution.
    pub components: Vec<Value>,
    /// pending, resolved or blocked.
    pub resolution: String,
    /// pending, ready, failed, blocked or not_applicable.
    pub preview_state: String,
    /// Human-readable source/render constraints.
    pub diagnostics: Vec<String>,
    /// Validated media outputs for this selection.
    pub media: Vec<Media>,
}

/// A content-verified local media file and its semantic view.
#[derive(Debug, Serialize, Deserialize)]
pub struct Media {
    /// Relative package path.
    pub path: String,
    /// Semantic camera view.
    pub view: String,
    /// Encoded file length in bytes.
    pub bytes: u64,
    /// Pixel dimensions.
    pub size: u32,
}

/// A reusable world object, distinct from gameplay inventory records.
#[derive(Debug, Serialize, Deserialize)]
pub struct Prop {
    /// Stable native resource identity, scoped to the prop collection.
    pub id: String,
    /// Human-readable name derived from the native resource spelling.
    pub label: String,
    /// Native entity or mesh resource.
    pub resource: ResourceKey,
    /// `assembly` for a native entity, `mesh` for an individual mesh resource.
    pub kind: String,
    /// Catalogue browsing category; native paths remain authoritative.
    pub category: String,
    /// Exact native appearances of this resource.
    pub selections: Vec<String>,
    /// Individual mesh prop identities used by this assembly.
    pub parts: Vec<String>,
    /// Native assemblies that contain this individual mesh.
    pub assemblies: Vec<String>,
    /// Discovery, native binding and assembly diagnostics.
    pub diagnostics: Vec<String>,
}

impl Catalogue {
    /// Refreshes independently counted discovery/resolution/media totals.
    pub fn update_coverage(&mut self) {
        let mut states = BTreeMap::<String, usize>::new();
        for selection in self.selections.values() {
            *states.entry(selection.preview_state.clone()).or_default() += 1;
        }
        self.coverage["item_records"] = self.items.len().into();
        self.coverage["visual_assets"] = self.assets.len().into();
        self.coverage["clothing_resources"] = self
            .assets
            .values()
            .filter(|asset| asset.roles.iter().any(|role| role == "clothing"))
            .count()
            .into();
        self.coverage["props"] = self.props.len().into();
        self.coverage["prop_assemblies"] = self
            .props
            .iter()
            .filter(|p| p.kind == "assembly")
            .count()
            .into();
        self.coverage["prop_meshes"] = self
            .props
            .iter()
            .filter(|p| p.kind == "mesh")
            .count()
            .into();
        self.coverage["customization_controls"] = self.controls.len().into();
        self.coverage["authored_choices"] = self.choices.len().into();
        self.coverage["selections"] = self.selections.len().into();
        self.coverage["resolved_selections"] = self
            .selections
            .values()
            .filter(|s| s.resolution == "resolved")
            .count()
            .into();
        let mut resolution_states = BTreeMap::<String, usize>::new();
        for selection in self.selections.values() {
            *resolution_states
                .entry(selection.resolution.clone())
                .or_default() += 1;
        }
        self.coverage["blocked_selections"] = resolution_states
            .get("blocked")
            .copied()
            .unwrap_or(0)
            .into();
        self.coverage["resolution_states"] =
            serde_json::to_value(resolution_states).expect("string/count map");
        self.coverage["preview_states"] = serde_json::to_value(states).expect("string/count map");
    }
}
