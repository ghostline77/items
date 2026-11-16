//! Reviewable family recipes and resumable bulk rendering over a built catalogue.
use crate::{
    model::{Catalogue, Control},
    preview::{self, NeutralHead, PreviewRequest, RenderOutcome, RenderSummary},
    publish,
};
use anyhow::{Context, Result, ensure};
use ghostline_red::assets::ResourceKey;
use ghostline_renderer::{GameSource, View};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

/// Native asset families selectable for a render plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    /// Native garments, including NPC-only clothing without equipment records.
    Clothing,
    /// World props, complete native assemblies and individual mesh resources.
    Props,
    /// All catalogue selections, including independent native resources.
    All,
    /// Hair and its FPP variants, with neutral head context.
    Hair,
    /// Equipment records, including clothing, weapons and attachments.
    Equipment,
    /// Facial parts, eyes, makeup, tattoos and other head selections.
    Head,
    /// Body parts, skin, tattoos and anatomy selections.
    Body,
    /// Hands, nails and arm selections.
    Arms,
    /// Independent native selections without a control or equipment owner.
    Other,
}

/// An explicit preview context, independent of the native selection identity.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    /// Native context selections that must remain inside the subject frame.
    #[serde(default)]
    pub framing_references: Vec<String>,
    /// Frame one native X half of a paired detail, retaining all geometry.
    #[serde(default)]
    pub subject_side: Option<bool>,
    /// Authored native bone axes for isolated rigid character attachments.
    #[serde(default)]
    pub native_frame: Option<ghostline_renderer::NativeBoneFrame>,
    /// Explicit native mesh LOD masks, keyed by resource hash.
    #[serde(default)]
    pub mesh_lods: BTreeMap<String, u64>,
    /// Evaluate the owning native rig instead of isolated bind-pose geometry.
    #[serde(default)]
    pub native_assembly: bool,
    /// Explicit external placement inputs frozen for a standalone native assembly.
    #[serde(default)]
    pub origin_anchors: BTreeSet<String>,
    /// Explicit native visibility state, such as scope masks outside aiming.
    #[serde(default)]
    pub visibility_parameters: std::collections::BTreeMap<String, bool>,
    /// Square output dimensions.
    pub size: u32,
    /// Native selection references retaining their authored materials.
    pub references: Vec<String>,
    /// Neutral player head frame, when relevant.
    pub neutral_head: Option<NeutralHead>,
    /// Native body assemblies displayed with neutral materials.
    pub neutral_assemblies: Vec<String>,
    /// Hero/front/back views appropriate to the family.
    pub views: Vec<View>,
    /// Frame the subject instead of the complete reference body.
    pub subject_frame: bool,
    /// Focus one native mesh instance for detail previews.
    pub subject_part: Option<usize>,
    /// Minimum radius for mounted details, in metres.
    #[serde(default)]
    pub subject_min_radius: Option<f32>,
    /// Revision for corrected material behavior in this recipe.
    #[serde(default)]
    pub material_revision: Option<u32>,
    /// Optional lower/upper fractions of subject height for detail framing.
    #[serde(default)]
    pub subject_crop: Option<[f32; 2]>,
}

/// A selection and its established family preview recipe.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedSelection {
    /// Exact catalogue selection ID.
    pub selection: String,
    /// Native ownership used to select the recipe.
    pub family: Family,
    /// Owning control IDs, retained as recipe provenance.
    pub controls: Vec<String>,
    /// Explicit output and reference context.
    pub recipe: Recipe,
}

/// Native binding failures that still need resolution before rendering.
#[derive(Debug, Serialize, Deserialize)]
pub struct BindingFailure {
    /// Exact catalogue selection ID.
    pub selection: String,
    /// Source-backed failure details; never a missing-recipe exclusion.
    pub diagnostics: Vec<String>,
}

/// A portable, editable render manifest generated from published catalogue data.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderPlan {
    /// Contract marker.
    pub format: String,
    /// Plan contract version.
    pub version: u32,
    /// Catalogue source declarations against which the plan was generated.
    pub sources: BTreeMap<String, String>,
    /// Requested native family scope.
    pub family: Family,
    /// Resolved selections, each with a concrete preview recipe.
    pub jobs: Vec<PlannedSelection>,
    /// Unresolved bindings retained as outstanding work.
    pub binding_failures: Vec<BindingFailure>,
    /// Internal, nonvisual companion selections retained as technical metadata.
    pub metadata_only: Vec<String>,
    /// Native evidence explaining why an entry has no 3D preview job.
    #[serde(default)]
    pub metadata_only_reasons: BTreeMap<String, String>,
}

fn frame(control: &Control) -> Option<NeutralHead> {
    for (path, frame) in [
        (
            "base/gameplay/gui/fullscreen/main_menu/female_cco.inkcharcustomization",
            NeutralHead::Female,
        ),
        (
            "ep1/gameplay/gui/fullscreen/main_menu/female_cco_ep1.inkcharcustomization",
            NeutralHead::Female,
        ),
        (
            "base/gameplay/gui/fullscreen/main_menu/male_cco.inkcharcustomization",
            NeutralHead::Male,
        ),
        (
            "ep1/gameplay/gui/fullscreen/main_menu/male_cco_ep1.inkcharcustomization",
            NeutralHead::Male,
        ),
    ] {
        if control.source.native_hash() == ResourceKey::parse(path).native_hash() {
            return Some(frame);
        }
    }
    None
}

// Linked skin-colour controls use the owning native root's authored palette.
// Match actual component appearance identities, then the palette identity in
// the indexed appearance definition; do not infer a body frame from filenames.
fn palette_reference(
    c: &Catalogue,
    owners: &[&Control],
    selection: &str,
    name: &str,
) -> Option<String> {
    let selected = c.selections.get(selection)?;
    for owner in owners {
        let Some(palette) = c.controls.iter().find(|control| {
            control.source == owner.source && control.native.name.as_deref() == Some(name)
        }) else {
            continue;
        };
        let choices: Vec<_> = c
            .choices
            .iter()
            .filter(|choice| choice.control == palette.id)
            .filter_map(|choice| choice.selection.as_ref())
            .filter(|id| {
                c.selections
                    .get(*id)
                    .is_some_and(|s| s.resolution == "resolved")
            })
            .collect();
        for id in &choices {
            let candidate = &c.selections[*id];
            let matches =
                candidate
                    .components
                    .iter()
                    .filter_map(|component| {
                        component
                            .get("mesh_appearance")
                            .and_then(serde_json::Value::as_str)
                    })
                    .filter(|appearance| !appearance.is_empty() && *appearance != "default")
                    .any(|appearance| {
                        selected.components.iter().any(|component| {
                            component["mesh_appearance"].as_str() == Some(appearance)
                        }) || selected
                            .appearance
                            .as_deref()
                            .is_some_and(|value| value.ends_with(&format!("__{appearance}")))
                    });
            if matches {
                return Some((*id).clone());
            }
        }
        if let Some(id) = choices.first() {
            return Some((*id).clone());
        }
    }
    None
}

fn arm_base_reference(c: &Catalogue, owners: &[&Control], selection: &str) -> Option<String> {
    for owner in owners {
        let name = owner.native.name.as_deref()?;
        if !name.contains("cyberware01") {
            continue;
        }
        let base_name = name.replace("cyberware01", "base");
        let base = c.controls.iter().find(|control| {
            control.source == owner.source && control.native.name.as_deref() == Some(&base_name)
        })?;
        let choice = c.choices.iter().find(|choice| {
            choice.control == owner.id && choice.selection.as_deref() == Some(selection)
        })?;
        let index = choice.data["index"].as_u64().unwrap_or(0);
        return c
            .choices
            .iter()
            .find(|candidate| {
                candidate.control == base.id
                    && candidate.data["index"].as_u64().unwrap_or(0) == index
            })
            .and_then(|choice| choice.selection.clone());
    }
    None
}

/// Establishes a concrete recipe for every resolved selection in the requested scope.
///
/// # Errors
/// Returns an error for unfinished bindings, invalid size or a missing native body baseline.
pub fn plan(c: &Catalogue, family: Family, size: u32) -> Result<RenderPlan> {
    ensure!((64..=4096).contains(&size), "size must be 64..4096");
    ensure!(
        !c.selections.values().any(|s| s.resolution == "pending"),
        "catalogue build has unfinished bindings"
    );
    let controls: BTreeMap<_, _> = c.controls.iter().map(|v| (v.id.as_str(), v)).collect();
    let mut owners = BTreeMap::<&str, Vec<&Control>>::new();
    for choice in &c.choices {
        if let Some(id) = choice.selection.as_deref()
            && let Some(&control) = controls.get(choice.control.as_str())
        {
            let entry = owners.entry(id).or_default();
            if !entry.iter().any(|v| v.id == control.id) {
                entry.push(control);
            }
        }
    }
    let equipment: BTreeSet<_> = c
        .items
        .iter()
        .flat_map(|v| v.selections.iter().map(String::as_str))
        .collect();
    let weapons: BTreeSet<_> = c
        .items
        .iter()
        .filter(|item| {
            item.item_type
                .as_deref()
                .is_some_and(|kind| kind.starts_with("Wea_"))
        })
        .flat_map(|item| item.selections.iter().map(String::as_str))
        .collect();
    let blades: BTreeSet<_> = c
        .items
        .iter()
        .filter(|item| {
            matches!(
                item.item_type.as_deref(),
                Some(
                    "Wea_Knife" | "Wea_Katana" | "Wea_LongBlade" | "Wea_Machete" | "Wea_Chainsword"
                )
            )
        })
        .flat_map(|item| item.selections.iter().map(String::as_str))
        .collect();
    let mut bodies = BTreeMap::<u8, String>::new();
    let mut arms = BTreeMap::<u8, String>::new();
    let mut eyes = BTreeMap::<u8, String>::new();
    let mut heads = BTreeMap::<u8, String>::new();
    for control in &c.controls {
        let baseline = match control.native.name.as_deref() {
            Some("body_color") => &mut bodies,
            Some("h_default_arms_colors_tpp" | "h_default_arms_colors") => &mut arms,
            Some("eyes_color") => &mut eyes,
            Some("skin_color") => &mut heads,
            _ => continue,
        };
        if let Some(frame) = frame(control) {
            let key = u8::from(frame == NeutralHead::Male);
            if let Some(id) = c
                .choices
                .iter()
                .filter(|v| v.control == control.id)
                .filter_map(|v| v.selection.as_ref())
                .find(|id| {
                    c.selections
                        .get(*id)
                        .is_some_and(|s| s.resolution == "resolved")
                })
            {
                baseline.entry(key).or_insert_with(|| id.clone());
            }
        }
    }
    let mut jobs = Vec::new();
    let mut binding_failures = Vec::new();
    let mut metadata_only = Vec::new();
    let mut metadata_only_reasons = BTreeMap::new();
    for (id, selection) in &c.selections {
        let owners = owners.get(id.as_str()).cloned().unwrap_or_default();
        if selection.resolution == "resolved"
            && (selection.context["material_bindings"]
                .as_array()
                .is_some_and(|bindings| {
                    !bindings.is_empty()
                        && bindings.iter().all(|binding| {
                            binding["binding"]["shadow_only"] == true
                                || binding["binding"]["non_visual"] == true
                        })
                })
                || selection.preview_state == "not_applicable"
                || selection.diagnostics.iter().any(|diagnostic| {
                    diagnostic == "render: invalid input: assembly has no enabled visible geometry"
                }))
        {
            metadata_only.push(id.clone());
            metadata_only_reasons.insert(
                id.clone(),
                "Native binding resolution found no enabled visible mesh components".into(),
            );
            continue;
        }
        // These native controls drive the player's invisible FPP hair/shadow
        // companions. The visible hairstyle is catalogued by hair_color instead.
        if !owners.is_empty()
            && owners
                .iter()
                .all(|v| v.native.ui_slot.as_deref() == Some("hair_color_fpp"))
        {
            metadata_only.push(id.clone());
            metadata_only_reasons.insert(
                id.clone(),
                "Native hair_color_fpp control supplies an invisible hair/shadow companion".into(),
            );
            continue;
        }
        let asset_family = if selection.context["prop"].is_string() {
            Family::Props
        } else if owners.iter().any(|v| {
            matches!(
                v.native.ui_slot.as_deref(),
                Some("hair_color" | "hair_color_fpp")
            )
        }) {
            Family::Hair
        } else if selection.context["clothing"] == true {
            Family::Clothing
        } else if equipment.contains(id.as_str()) {
            Family::Equipment
        } else if owners.iter().any(|v| v.native.domain == "head") {
            Family::Head
        } else if owners.iter().any(|v| v.native.domain == "body") {
            Family::Body
        } else if owners.iter().any(|v| v.native.domain == "arms") {
            Family::Arms
        } else {
            Family::Other
        };
        if family != Family::All && family != asset_family {
            continue;
        }
        if selection.resolution != "resolved" {
            binding_failures.push(BindingFailure {
                selection: id.clone(),
                diagnostics: selection.diagnostics.clone(),
            });
            continue;
        }
        let frame = owners
            .iter()
            .find_map(|v| frame(v))
            .unwrap_or(NeutralHead::Female);
        let mut recipe = Recipe {
            framing_references: Vec::new(),
            subject_side: None,
            native_frame: None,
            mesh_lods: BTreeMap::new(),
            native_assembly: selection.context["prop_kind"] == "assembly",
            origin_anchors: serde_json::from_value(
                selection.context["standalone_origin_anchors"].clone(),
            )
            .unwrap_or_default(),
            visibility_parameters: if selection
                .resource
                .to_string()
                .replace('/', "\\")
                .contains("\\attachments\\scope\\")
            {
                std::collections::BTreeMap::from([("visibleWhenAiming".into(), false)])
            } else {
                Default::default()
            },
            size,
            references: Vec::new(),
            neutral_head: None,
            neutral_assemblies: Vec::new(),
            views: vec![View::Hero, View::Front, View::Back],
            subject_frame: false,
            subject_part: None,
            subject_min_radius: None,
            material_revision: None,
            subject_crop: None,
        };
        if selection.resource.native_hash() == 6_865_793_532_744_653_722 {
            recipe.material_revision = Some(1);
        }
        if selection.resource.native_hash() == 10_061_218_191_762_617_435
            && selection
                .appearance
                .as_deref()
                .is_some_and(|appearance| appearance.starts_with("glow_tattoo_"))
        {
            recipe.material_revision = Some(1);
        }
        match asset_family {
            Family::Hair => recipe.neutral_head = Some(frame),
            Family::Head => {
                recipe.neutral_head = Some(frame);
                recipe.subject_frame = true;
                recipe.views = vec![View::Hero, View::Front];
            }
            Family::Body | Family::Arms => {
                let body = bodies
                    .get(&u8::from(frame == NeutralHead::Male))
                    .context("native body-color baseline is missing from the built catalogue")?;
                if body != id {
                    recipe.neutral_assemblies.push(body.clone());
                }
                recipe.subject_frame = true;
            }
            Family::All | Family::Equipment | Family::Clothing | Family::Other | Family::Props => {}
        }
        if owners.iter().any(|v| {
            v.native.ui_slot.as_deref() == Some("nails_color")
                || v.native.name.as_deref() == Some("nails_color_fpp")
        }) {
            let hand = arms
                .get(&u8::from(frame == NeutralHead::Male))
                .context("native default arm/hand baseline is missing from the built catalogue")?;
            recipe.neutral_assemblies = vec![hand.clone()];
            recipe.subject_part = Some(0);
            recipe.views = vec![View::Hero, View::Front, View::Back];
        }
        if owners.iter().any(|v| {
            matches!(
                v.native.ui_slot.as_deref(),
                Some("eyes_color" | "eyebrows_color" | "eyelash_color")
            )
        }) {
            recipe.subject_part = Some(0);
        }
        // Skin selections supply their own head surface. Teeth require an
        // unobstructed dental close-up rather than an occluding closed mouth.
        if owners.iter().any(|v| {
            matches!(
                v.native.ui_slot.as_deref(),
                Some("skin_color" | "skin_type" | "teeth")
            )
        }) {
            recipe.neutral_head = None;
        }
        if asset_family == Family::Head
            && recipe.neutral_head.is_some()
            && let Some(head) = heads.get(&u8::from(frame == NeutralHead::Male))
            && head != id
        {
            recipe.neutral_head = None;
            recipe.neutral_assemblies.push(head.clone());
        }
        if asset_family == Family::Head
            && let Some(eye) = eyes.get(&u8::from(frame == NeutralHead::Male))
            && eye != id
            && !owners
                .iter()
                .any(|v| v.native.ui_slot.as_deref() == Some("teeth"))
        {
            recipe.neutral_assemblies.push(eye.clone());
        }
        if owners.iter().any(|v| {
            matches!(
                v.native.ui_slot.as_deref(),
                Some("piercings_color" | "cyberware" | "facial_tattoo" | "scars")
            )
        }) {
            recipe.views = vec![View::Front, View::Left, View::Right];
        }
        if weapons.contains(id.as_str()) {
            recipe.views = vec![View::Hero, View::Left, View::Front];
        }
        if blades.contains(id.as_str()) {
            recipe.views = vec![View::Hero, View::Left, View::Right];
        }
        let slot = |names: &[&str]| {
            owners.iter().any(|owner| {
                owner
                    .native
                    .ui_slot
                    .as_deref()
                    .is_some_and(|value| names.contains(&value))
            })
        };
        let nails = slot(&["nails_color"])
            || owners
                .iter()
                .any(|owner| owner.native.data["link"]["$value"] == "nails_color");
        if nails {
            let hand = arms
                .get(&u8::from(frame == NeutralHead::Male))
                .context("native default arm/hand baseline is missing from the built catalogue")?;
            recipe.neutral_assemblies = vec![hand.clone()];
            for owner in &owners {
                let native_base = match owner.native.name.as_deref() {
                    Some("u_launcher_nails_color") => Some("u_launcher_arms_colors_base"),
                    Some("u_mantise_nails_color") => Some("u_mantis_arms_colors_base"),
                    Some("nails_color_fpp") => Some("h_default_arms_colors_fpp"),
                    _ => None,
                };
                if let Some(name) = native_base
                    && let Some(reference) = palette_reference(c, &owners, id, name)
                {
                    recipe.neutral_assemblies = vec![reference];
                    if let Some(prefix) = name.strip_suffix("base")
                        && let Some(surface) =
                            palette_reference(c, &owners, id, &format!("{prefix}cyberware01"))
                    {
                        recipe.neutral_assemblies.push(surface);
                    }
                    break;
                }
            }
            recipe.subject_part = Some(0);
        }
        if (asset_family == Family::Body || asset_family == Family::Arms) && !nails {
            let fpp = slot(&["fpp_body_color", "fpp_body_tattoo", "fpp_nipples"]);
            let baseline_name = if fpp { "fpp_body_color" } else { "body_color" };
            recipe.neutral_assemblies.clear();
            if !slot(&["body_color", "fpp_body_color", "nails_color"])
                && let Some(reference) = palette_reference(c, &owners, id, baseline_name)
                    .filter(|reference| reference != id)
            {
                recipe.references.push(reference);
            }
        }
        if asset_family == Family::Arms && !nails {
            recipe.references.clear();
            recipe.subject_part = Some(0);
            if let Some(reference) =
                arm_base_reference(c, &owners, id).filter(|reference| reference != id)
            {
                recipe.references.push(reference);
            }
        }
        if nails || slot(&["nipples", "fpp_nipples", "genitals", "pubic_hair_color"]) {
            recipe.views = vec![View::Hero, View::Front];
        }
        // The FPP nipple surface has its own authored shape. The default FPP
        // torso covers it in the bind pose, so preview that surface directly.
        if slot(&["fpp_nipples"]) {
            recipe.references.clear();
        }
        if slot(&["flat_feet", "lifted_feet"]) {
            recipe.subject_part = Some(0);
            recipe.subject_crop = Some([0., 0.3]);
        }
        if slot(&["scars", "finalSceneBruises"]) {
            recipe.neutral_head = None;
            recipe
                .neutral_assemblies
                .retain(|reference| !heads.values().any(|head| head == reference));
            if let Some(reference) = palette_reference(c, &owners, id, "skin_type_01")
                .filter(|reference| reference != id)
            {
                recipe.references.push(reference);
            }
        }
        if slot(&["neck"])
            || owners
                .iter()
                .any(|owner| owner.native.name.as_deref() == Some("tpp_head_proxy"))
        {
            recipe.neutral_head = None;
            recipe.neutral_assemblies.clear();
            recipe.references.clear();
            recipe.views = if slot(&["neck"]) {
                vec![View::Hero, View::Back]
            } else {
                vec![View::Hero, View::Front, View::Back]
            };
        }
        if owners
            .iter()
            .any(|owner| owner.native.ui_slot.as_deref() == Some("eyebrows_color"))
        {
            recipe.material_revision = Some(3);
        }
        if owners
            .iter()
            .any(|owner| owner.native.ui_slot.as_deref() == Some("makeupLips_color"))
        {
            recipe.material_revision = Some(2);
        }
        if owners
            .iter()
            .any(|owner| owner.native.ui_slot.as_deref() == Some("piercings_color"))
        {
            recipe.subject_min_radius = Some(0.035);
            recipe.views = vec![View::Hero, View::Front, View::Side];
        }
        if selection.resource.native_hash() == 8_763_825_068_884_944_178
            && selection.appearance.as_deref() == Some("w_sledge_hammer_boss")
        {
            // The high-detail native blade is compressed in its animation rest
            // shape; the authored LOD 2 supplies the static extended hammer that
            // matches the boss decals, without inventing a scale or translation.
            recipe.mesh_lods.insert("13395000390552617290".into(), 2);
        }
        if selection.resource.to_string().contains("fur_shell.mesh")
            || selection
                .resource
                .to_string()
                .contains("landfill_sparse_lp.mesh")
        {
            // Recoloring and detail coverage are part of the durable recipe,
            // so previews made before these shader inputs were supported expire.
            recipe.material_revision = Some(4);
        }
        if [
            7_610_293_935_767_277_935,
            9_808_744_606_840_698_264,
            8_031_727_031_483_980_991,
        ]
        .contains(&selection.resource.native_hash())
        {
            recipe.material_revision = Some(5);
        }
        if asset_family == Family::Clothing
            && (selection.resource.to_string().contains("specs")
                || selection.resource.to_string().contains("glasses"))
        {
            recipe.native_frame = Some(ghostline_renderer::NativeBoneFrame {
                rig: ghostline_red::assets::ResourceKey::parse(
                    r"base\characters\base_entities\woman_base\woman_base.rig",
                ),
                bone: "Head".into(),
            });
            recipe.material_revision = Some(9);
        }
        if asset_family != Family::Clothing
            && (selection.resource.to_string().contains("specs")
                || selection.resource.to_string().contains("glasses"))
        {
            // Loose world/quest props retain their authored world frame.
            recipe.material_revision = Some(11);
        }
        if selection
            .resource
            .to_string()
            .contains("strongarms_knuckles")
        {
            recipe.subject_frame = true;
            recipe.subject_part = Some(0);
            recipe.views = vec![View::Hero, View::Front, View::Back];
            recipe.material_revision = Some(9);
        }
        // The native marker includes a helper triangle 1.9 km below the
        // arrow. Keep its geometry but frame the visible, above-ground part.
        let marker = selection.resource.native_hash() == 16_108_600_794_980_572_149
            || selection.context["material_bindings"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|binding| {
                    serde_json::from_value::<ghostline_red::assets::ResourceKey>(
                        binding["binding"]["geometry"].clone(),
                    )
                    .is_ok_and(|key| key.native_hash() == 16_108_600_794_980_572_149)
                });
        if marker {
            recipe.subject_frame = true;
            recipe.subject_crop = Some([0.94, 1.]);
            recipe.material_revision = Some(9);
        }
        if slot(&["scars", "finalSceneBruises"])
            || selection.resource.to_string().contains("hx_000__scars")
            || selection.resource.to_string().contains("hx_000__beatenup")
            || selection.resource.to_string().contains("bottle")
        {
            recipe.material_revision = Some(9);
        }
        if slot(&["scars", "finalSceneBruises"])
            || selection.resource.to_string().contains("hx_000__scars")
            || selection.resource.to_string().contains("hx_000__beatenup")
        {
            recipe.material_revision = Some(10);
        }
        let native_path = selection.resource.to_string();
        if native_path.contains("cuff")
            || native_path.contains("gloves__")
            || native_path.contains("mantisblades_edge")
        {
            recipe.subject_side = Some(true);
            recipe.subject_frame = true;
            recipe.material_revision = Some(12);
        }
        if weapons.contains(id.as_str()) {
            recipe.views = vec![View::Hero, View::Left, View::Right];
            recipe.material_revision = Some(12);
        }
        if slot(&["eyelash_color"]) {
            // References use complementary native chunk masks; retain eye
            // context while the selected lashes keep their authored profile.
            recipe.material_revision = Some(15);
        }
        if slot(&[
            "skin_type",
            "skin_color",
            "body_scars",
            "body_tattoo",
            "fpp_body_tattoo",
            "pubic_hair_color",
            "body_color",
            "fpp_body_color",
            "lifted_feet",
            "flat_feet",
            "nipples",
            "fpp_nipples",
        ]) {
            recipe.material_revision = Some(12);
        }
        if slot(&["nipples", "fpp_nipples"]) {
            recipe.subject_side = Some(true);
            recipe.views = vec![View::Hero, View::Front, View::Side];
            recipe.material_revision = Some(13);
        }
        if slot(&["pubic_hair_color"]) {
            // Catalogue preview policy: vaginal context on the female frame,
            // penile context on the male frame. This is preview anatomy, not a
            // compatibility restriction on the native hair attachment variants.
            let genital_control = match frame {
                NeutralHead::Female => "genitals_01",
                NeutralHead::Male => "genitals_02",
            };
            let body = recipe
                .references
                .first()
                .map_or(id.as_str(), String::as_str);
            let anatomy = palette_reference(c, &owners, body, genital_control)
                .with_context(|| format!("native genital preview context missing for {id}"))?;
            recipe.references.push(anatomy.clone());
            recipe.framing_references.push(anatomy);
            recipe.subject_min_radius = Some(0.11);
            recipe.material_revision = Some(18);
        }
        jobs.push(PlannedSelection {
            selection: id.clone(),
            family: asset_family,
            controls: owners.iter().map(|v| v.id.clone()).collect(),
            recipe,
        });
    }
    Ok(RenderPlan {
        format: "ghostline-items-render-plan".into(),
        version: 1,
        sources: c.sources.clone(),
        family,
        jobs,
        binding_failures,
        metadata_only,
        metadata_only_reasons,
    })
}

/// GPU worker and cancellation settings for a bulk run.
pub struct RunOptions<'a> {
    /// Concurrent GPU workers, without an arbitrary upper limit.
    pub workers: usize,
    /// Cooperative cancellation marker.
    pub cancel: Option<&'a Path>,
}

/// Runs a manifest range concurrently and publishes successful output and failures.
///
/// # Errors
/// Returns an error for invalid plans, mismatched sources or publication failures.
pub fn run(
    c: &mut Catalogue,
    source: &GameSource,
    plan: &RenderPlan,
    package: &Path,
    work: &Path,
    range: std::ops::Range<usize>,
    execution: RunOptions<'_>,
) -> Result<RenderSummary> {
    ensure!(
        plan.format == "ghostline-items-render-plan" && plan.version == 1,
        "invalid render plan"
    );
    ensure!(
        plan.sources == c.sources,
        "render plan catalogue sources differ"
    );
    ensure!(
        range.start <= range.end && range.start <= plan.jobs.len(),
        "invalid render plan range"
    );
    let mut publication_changed = false;
    let mut invalidated_media = BTreeSet::new();
    for id in &plan.metadata_only {
        if let Some(selection) = c.selections.get_mut(id) {
            publication_changed |=
                selection.preview_state != "not_applicable" || !selection.media.is_empty();
            invalidated_media.extend(selection.media.iter().map(|media| media.path.clone()));
            selection.media.clear();
            selection.preview_state = "not_applicable".into();
            selection.diagnostics.retain(|d| !d.starts_with("render: "));
            selection.context["preview_omission"] =
                serde_json::json!(plan.metadata_only_reasons.get(id));
        }
    }
    let mut unique = BTreeSet::new();
    let mut reused = Vec::new();
    let mut groups = BTreeMap::<String, (Recipe, Vec<String>)>::new();
    for job in plan
        .jobs
        .iter()
        .skip(range.start)
        .take(range.end.saturating_sub(range.start))
    {
        ensure!(
            unique.insert(&job.selection),
            "duplicate selection in render plan: {}",
            job.selection
        );
        ensure!(
            (64..=4096).contains(&job.recipe.size)
                && !job.recipe.views.is_empty()
                && job.recipe.views.len() <= 3,
            "invalid render recipe"
        );
        let unique_views: BTreeSet<_> = job.recipe.views.iter().map(|v| format!("{v:?}")).collect();
        ensure!(
            unique_views.len() == job.recipe.views.len(),
            "duplicate semantic views in render recipe"
        );
        ensure!(
            c.selections
                .get(&job.selection)
                .is_some_and(|s| s.resolution == "resolved"),
            "selection has unresolved bindings: {}",
            job.selection
        );
        let selection = c
            .selections
            .get_mut(&job.selection)
            .expect("validated selection");
        let render = &selection.context["render"];
        let changed = render["subject_side"] != serde_json::json!(job.recipe.subject_side)
            || render
                .get("framing_references")
                .unwrap_or(&serde_json::json!([]))
                != &serde_json::json!(job.recipe.framing_references)
            || render["native_frame"] != serde_json::json!(job.recipe.native_frame)
            || render.get("mesh_lods").unwrap_or(&serde_json::json!({}))
                != &serde_json::json!(job.recipe.mesh_lods)
            || render["native_assembly"].as_bool().unwrap_or(false) != job.recipe.native_assembly
            || render
                .get("origin_anchors")
                .unwrap_or(&serde_json::json!([]))
                != &serde_json::json!(job.recipe.origin_anchors)
            || render["neutral_assemblies"] != serde_json::json!(job.recipe.neutral_assemblies)
            || render["references"] != serde_json::json!(job.recipe.references)
            || render["neutral_head"]["frame"] != serde_json::json!(job.recipe.neutral_head)
            || render["subject_frame"] != serde_json::json!(job.recipe.subject_frame)
            || render["subject_part"] != serde_json::json!(job.recipe.subject_part)
            || render["subject_min_radius"] != serde_json::json!(job.recipe.subject_min_radius)
            || render["material_revision"] != serde_json::json!(job.recipe.material_revision)
            || render
                .get("visibility_parameters")
                .unwrap_or(&serde_json::json!({}))
                != &serde_json::json!(job.recipe.visibility_parameters)
            || render["subject_crop"] != serde_json::json!(job.recipe.subject_crop)
            || render["size"] != serde_json::json!(job.recipe.size)
            || render["views"] != serde_json::json!(job.recipe.views);
        if changed && !selection.media.is_empty() {
            publication_changed = true;
            invalidated_media.extend(selection.media.iter().map(|media| media.path.clone()));
            selection.media.clear();
            selection.preview_state = "pending".into();
            selection
                .context
                .as_object_mut()
                .map(|c| c.remove("render"));
        }
        // Published images survive executable upgrades when their native recipe
        // is unchanged. Renderer build fingerprints still protect partial work.
        if !changed
            && selection.preview_state == "ready"
            && selection.media.len() == job.recipe.views.len()
            && selection.media.iter().all(|media| {
                media.size == job.recipe.size
                    && fs::metadata(package.join(&media.path))
                        .is_ok_and(|file| file.is_file() && file.len() == media.bytes)
            })
        {
            reused.push(RenderOutcome {
                selection: job.selection.clone(),
                state: "ready".into(),
                error: None,
                written_frames: 0,
                reused_frames: selection.media.len(),
            });
            continue;
        }
        // A ready record may have missing/corrupt media after an interrupted
        // publication or deliberate removal of a broken preview. Do not try to
        // republish those stale references before the replacement job runs.
        publication_changed |= selection.preview_state == "ready" || !selection.media.is_empty();
        invalidated_media.extend(selection.media.iter().map(|media| media.path.clone()));
        selection.media.clear();
        selection.preview_state = "pending".into();
        groups
            .entry(serde_json::to_string(&job.recipe)?)
            .or_insert_with(|| (job.recipe.clone(), Vec::new()))
            .1
            .push(job.selection.clone());
    }
    ensure!(!unique.is_empty(), "render plan range contains no jobs");
    eprintln!(
        "Reusing {} published previews with unchanged recipes",
        reused.len()
    );
    fs::create_dir_all(work)?;
    let media_base = fs::read(package.join("manifest.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|manifest| manifest["media_base"].as_str().map(str::to_owned))
        .unwrap_or_default();
    if publication_changed {
        // Removing references cannot invalidate the package's retained media.
        // Commit the new state before deleting only the files it supersedes.
        publish::checkpoint(c, package, &media_base)?;
    }
    for selection in c.selections.values() {
        for media in &selection.media {
            invalidated_media.remove(&media.path);
        }
    }
    for path in invalidated_media {
        ensure!(
            Path::new(&path)
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_))),
            "invalid media path"
        );
        match fs::remove_file(package.join(&path)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    let mut summary = RenderSummary {
        outcomes: reused,
        cancelled: false,
    };
    let checkpoint_clock = std::cell::Cell::new(std::time::Instant::now());
    for (_, (recipe, ids)) in groups {
        let result = preview::render(
            c,
            source,
            &ids,
            package,
            work,
            PreviewRequest {
                framing_references: &recipe.framing_references,
                subject_side: recipe.subject_side,
                native_frame: recipe.native_frame.clone(),
                mesh_lods: &recipe.mesh_lods,
                native_assembly: recipe.native_assembly,
                origin_anchors: &recipe.origin_anchors,
                workers: execution.workers,
                size: recipe.size,
                cancel: execution.cancel,
                references: &recipe.references,
                neutral_head: recipe.neutral_head,
                neutral_assemblies: &recipe.neutral_assemblies,
                views: &recipe.views,
                subject_frame: recipe.subject_frame,
                subject_part: recipe.subject_part,
                subject_min_radius: recipe.subject_min_radius,
                material_revision: recipe.material_revision,
                visibility_parameters: &recipe.visibility_parameters,
                checkpoint_clock: Some(&checkpoint_clock),
                subject_crop: recipe.subject_crop,
            },
        )?;
        summary.cancelled = result.cancelled;
        summary.outcomes.extend(result.outcomes);
        publish::atomic(
            &work.join("bulk-report.json"),
            &serde_json::to_vec_pretty(&serde_json::json!({
                "selected":unique.len(),"reported":summary.outcomes.len(),"cancelled":summary.cancelled,
                "ready":summary.outcomes.iter().filter(|v|v.state=="ready").count(),
                "failed":summary.outcomes.iter().filter(|v|v.state=="failed").count(),
                "not_applicable":summary.outcomes.iter().filter(|v|v.state=="not_applicable").count(),
                "outcomes":summary.outcomes,
            }))?,
        )?;
        if summary.cancelled {
            break;
        }
    }
    if !summary.cancelled {
        // Bindings are unchanged, retained images were previously published,
        // and every new image has already passed the renderer's atomic commit.
        publish::checkpoint(c, package, &media_base)?;
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn all_resolved_families_get_recipes_and_only_binding_failures_remain_outstanding() {
        let mut c:Catalogue=serde_json::from_value(json!({"format":"ghostline-items","version":1,"sources":{},"items":[],"assets":{},"controls":[],"choices":[],"selections":{},"customization_metadata":{},"coverage":{},"diagnostics":[]})).unwrap();
        for (label, domain, slot) in [
            ("hair", "head", "hair_color"),
            ("face", "head", "makeupEyes_color"),
            ("body_color", "body", "body_color"),
            ("h_default_arms_colors_tpp", "arms", ""),
            ("nails", "arms", "nails_color"),
        ] {
            let id = crate::build::add_selection(
                &mut c,
                ResourceKey::parse(&format!("base/{label}.app")),
                Some("first".into()),
                serde_json::json!({}),
            );
            c.selections.get_mut(&id).unwrap().resolution = "resolved".into();
            c.controls.push(serde_json::from_value(json!({"id":label,"source":{"Path":"base/gameplay/gui/fullscreen/main_menu/female_cco.inkcharcustomization"},"native":{"domain":domain,"source_index":0,"name":label,"native_class":"gameuiAppearanceInfo","ui_slot":slot,"data":{}},"choices":[format!("{label}/0")]})).unwrap());
            c.choices.push(serde_json::from_value(json!({"id":format!("{label}/0"),"control":label,"kind":"definitions","source_index":0,"label":label,"data":{},"selection":id})).unwrap());
        }
        let other = crate::build::add_selection(
            &mut c,
            ResourceKey::parse("base/other.ent"),
            None,
            json!({}),
        );
        c.selections.get_mut(&other).unwrap().resolution = "resolved".into();
        let missing = crate::build::add_selection(
            &mut c,
            ResourceKey::parse("base/missing.ent"),
            None,
            json!({}),
        );
        c.selections.get_mut(&missing).unwrap().resolution = "blocked".into();
        c.selections
            .get_mut(&missing)
            .unwrap()
            .diagnostics
            .push("missing native resource".into());
        let plan = plan(&c, Family::All, 1024).unwrap();
        assert_eq!(plan.jobs.len(), 6);
        assert_eq!(plan.binding_failures.len(), 1);
        assert_eq!(
            plan.jobs
                .iter()
                .find(|j| j.family == Family::Hair)
                .unwrap()
                .recipe
                .neutral_head,
            Some(NeutralHead::Female)
        );
        assert_eq!(
            plan.jobs
                .iter()
                .find(|j| j.selection.contains("nails.app"))
                .unwrap()
                .recipe
                .neutral_assemblies
                .len(),
            1
        );
        assert_eq!(
            plan.jobs
                .iter()
                .find(|j| j.family == Family::Head)
                .unwrap()
                .recipe
                .views
                .len(),
            2
        );
        assert!(plan.jobs.iter().any(|j| j.family == Family::Other));
        let decoded: RenderPlan =
            serde_json::from_slice(&serde_json::to_vec(&plan).unwrap()).unwrap();
        assert_eq!(decoded.jobs.len(), plan.jobs.len());
        let nails = plan
            .jobs
            .iter()
            .find(|j| j.selection.contains("nails.app"))
            .unwrap();
        assert_eq!(nails.recipe.subject_part, Some(0));
        assert!(nails.recipe.neutral_assemblies[0].contains("h_default_arms_colors_tpp"));
    }
}
