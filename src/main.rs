//! Build complete native catalogues, inspect entries and render previews.
use anyhow::{Context, Result, ensure};
use clap::{Args, Parser, Subcommand};
use ghostline_items::{
    build::{self, BuildInputs},
    bulk, clothing, preview, props, publish,
};
use ghostline_renderer::GameSource;
use std::{fs, path::PathBuf};

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Native equipment, character customization, furniture and prop catalogue"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Args)]
struct Game {
    #[arg(long)]
    game: PathBuf,
    #[arg(long)]
    schema: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct Selection {
    /// Known native item record name (for example Items.Vest_17_basic_02).
    #[arg(long)]
    item: Option<String>,
    /// Native entity or appearance resource path.
    #[arg(long)]
    resource: Option<String>,
    /// Exact authored appearance; ambiguity is reported with readable alternatives.
    #[arg(long)]
    appearance: Option<String>,
    /// Process every discovered selection.
    #[arg(long)]
    all: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum BuildScope {
    All,
    Items,
    Props,
    Clothing,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Update browser assets without regenerating catalogue data or images.
    RefreshUi {
        #[arg(long)]
        catalogue: PathBuf,
    },
    /// Build selected collections from native game data, including all bindings.
    Build {
        #[command(flatten)]
        game: Game,
        /// Exact compiled cache; normally tweakdb.bin or tweakdb_ep1.bin.
        #[arg(long)]
        tweakdb: Option<PathBuf>,
        /// Collections to discover; props/clothing do not require TweakDB.
        #[arg(long, value_enum, default_value = "all")]
        scope: BuildScope,
        /// Resume discovered metadata from an existing same-source catalogue,
        /// retrying blocked bindings and incomplete native placements.
        #[arg(long)]
        resume: bool,
        /// Native depot path dictionary (UTF-8 lines, optionally KARK compressed).
        #[arg(long)]
        resource_names: Option<PathBuf>,
        /// Limit prop discovery to these depot prefixes (repeatable).
        #[arg(long)]
        prop_prefix: Vec<String>,
        /// Native name dictionary (length-prefixed, optionally KARK compressed).
        #[arg(long)]
        tweak_names: Option<PathBuf>,
        #[arg(long, default_value = "en-us")]
        language: String,
        #[arg(long)]
        game_version: String,
        /// Empty directory or existing generated items package, outside the game.
        #[arg(long)]
        out: PathBuf,
    },
    /// Search native names, labels, resource paths and technical values.
    Search {
        #[arg(long)]
        catalogue: PathBuf,
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Emit an exact item/control/choice/selection as JSON.
    Show {
        #[arg(long)]
        catalogue: PathBuf,
        id: String,
    },
    /// Render selected resources through the shared native renderer.
    Render {
        #[arg(long)]
        catalogue: PathBuf,
        #[command(flatten)]
        game: Game,
        #[command(flatten)]
        selection: Selection,
        /// Persistent renderer batch/cache directory, outside the game/package.
        #[arg(long)]
        work: PathBuf,
        #[arg(long,default_value_t=512,value_parser=clap::value_parser!(u32).range(64..=4096))]
        size: u32,
        /// Stop cooperatively when this file exists.
        #[arg(long)]
        cancel_file: Option<PathBuf>,
        /// Native resource path for reference geometry.
        #[arg(long)]
        reference_resource: Option<String>,
        #[arg(long, requires = "reference_resource")]
        reference_appearance: Option<String>,
        /// Matte grey native base head for hair previews; choose the frame explicitly.
        #[arg(long, value_enum, conflicts_with = "reference_resource")]
        neutral_head: Option<preview::NeutralHead>,
    },
    /// Establish family recipes for a reviewable bulk render manifest.
    PlanRenders {
        #[arg(long)]
        catalogue: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, value_enum, default_value = "all")]
        family: bulk::Family,
        #[arg(long,default_value_t=1024,value_parser=clap::value_parser!(u32).range(64..=4096))]
        size: u32,
    },
    /// Run or resume a bounded range from a bulk render manifest.
    RenderBatch {
        /// Number of concurrent GPU render workers.
        #[arg(long, default_value_t = 16)]
        workers: usize,
        #[arg(long)]
        catalogue: PathBuf,
        #[command(flatten)]
        game: Game,
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        work: PathBuf,
        #[arg(long, default_value_t = 0)]
        offset: usize,
        /// Restore completed previews from this same-source render cache (repeatable).
        #[arg(long)]
        resume_from: Vec<PathBuf>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        cancel_file: Option<PathBuf>,
    },
    /// Verify catalogue references, manifest identity and media hashes.
    Verify {
        #[arg(long)]
        catalogue: PathBuf,
    },
    /// Write a hosted/offline copy, preserving all local media.
    Package {
        #[arg(long)]
        catalogue: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "")]
        media_base: String,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Build {
            game,
            tweakdb,
            tweak_names,
            language,
            game_version,
            out,
            scope,
            resume,
            resource_names,
            prop_prefix,
        } => {
            if matches!(scope, BuildScope::All | BuildScope::Items) {
                ensure!(
                    tweakdb.is_some(),
                    "--tweakdb is required for items/all scope"
                );
            }
            {
                ensure!(
                    resource_names.is_some(),
                    "--resource-names is required for native clothing/prop discovery"
                );
            }
            ensure!(
                language
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-'),
                "invalid language code"
            );
            publish::outside_game(&out, &game.game)?;
            let previous = if out.join("data/catalogue.json").is_file() {
                let start = std::time::Instant::now();
                eprintln!("Loading published catalogue to preserve previews...");
                let previous = publish::load(&out)?;
                eprintln!(
                    "Loaded {} published selections in {:.1}s",
                    previous.selections.len(),
                    start.elapsed().as_secs_f64()
                );
                Some(previous)
            } else {
                None
            };
            publish::initialize(&out)?;
            if resume {
                ensure!(scope == BuildScope::All, "--resume requires --scope all");
                let mut c = previous.context("--resume requires an existing catalogue")?;
                let source = GameSource::open(&game.game, game.schema.as_deref())?;
                ensure!(
                    c.sources.get("props_source_identity") == Some(&source.content_identity()),
                    "archive/schema identity changed; run a fresh build"
                );
                ensure!(
                    c.sources.get("game_version") == Some(&game_version)
                        && c.sources.get("language") == Some(&language),
                    "catalogue version/language changed; run a fresh build"
                );
                let mut retry = 0;
                for selection in c
                    .selections
                    .values_mut()
                    .filter(|selection| selection.resolution == "blocked")
                {
                    selection.resolution = "pending".into();
                    selection.preview_state = "pending".into();
                    selection
                        .diagnostics
                        .retain(|d| !d.starts_with("resolution: "));
                    retry += 1;
                }
                eprintln!(
                    "Resuming catalogue: retrying {retry} blocked selections; retaining completed discovery and previews"
                );
                props::resolve_assemblies(&mut c, &source)?;
                publish::write(&mut c, &out, "")?;
                publish::prune_media(&c, &out)?;
                println!("{}", serde_json::to_string_pretty(&c.coverage)?);
                return Ok(());
            }
            let mut c = if matches!(scope, BuildScope::Props | BuildScope::Clothing) {
                props::empty_catalogue(&game_version, &language)
            } else {
                build::build(&BuildInputs {
                    game: &game.game,
                    schema: game.schema.as_deref(),
                    tweakdb: tweakdb.as_deref().expect("validated item inputs"),
                    tweak_names: tweak_names.as_deref(),
                    language: &language,
                    game_version: &game_version,
                })?
            };
            if scope != BuildScope::Props {
                let source = GameSource::open(&game.game, game.schema.as_deref())?;
                clothing::extend(
                    &mut c,
                    &source,
                    resource_names.as_deref().expect("validated native names"),
                )?;
            }
            if matches!(scope, BuildScope::All | BuildScope::Props) {
                let source = GameSource::open(&game.game, game.schema.as_deref())?;
                props::extend(
                    &mut c,
                    &source,
                    &props::PropInputs {
                        resource_names: resource_names.as_deref().expect("validated prop inputs"),
                        prefixes: &prop_prefix,
                    },
                )?;
            }
            if let Some(mut previous) = previous.filter(|previous| {
                previous.sources.iter().all(|(key, value)| {
                    (key.starts_with("props_") && key != "props_source_identity")
                        || c.sources.get(key) == Some(value)
                })
            }) {
                for (id, selection) in &mut c.selections {
                    if let Some(prior) = previous.selections.remove(id).filter(|prior| {
                        preview::same_bind_pose_inputs(prior, selection)
                            && matches!(prior.preview_state.as_str(), "ready" | "not_applicable")
                    }) {
                        selection.media = prior.media;
                        selection.preview_state.clone_from(&prior.preview_state);
                        if let Some(render) = prior.context.get("render") {
                            selection.context["render"] = render.clone();
                        }
                        if let Some(reason) = prior.context.get("preview_omission") {
                            selection.context["preview_omission"] = reason.clone();
                        }
                    }
                }
            }
            publish::write(&mut c, &out, "")?;
            publish::prune_media(&c, &out)?;
            println!("{}", serde_json::to_string_pretty(&c.coverage)?);
        }
        Command::Search {
            catalogue,
            query,
            limit,
        } => {
            let c = publish::load(&catalogue)?;
            let needle = query.to_lowercase();
            let mut n = 0;
            for item in &c.items {
                if serde_json::to_string(item)?
                    .to_lowercase()
                    .contains(&needle)
                    && n < limit
                {
                    let variants: Vec<_> = item
                        .selections
                        .iter()
                        .filter_map(|id| c.selections.get(id))
                        .map(|v| readable_variant(v, &catalogue))
                        .collect();
                    println!(
                        "{}",
                        serde_json::json!({"kind":"item","name":item.native_name,"label":item.label,"native_id":item.native_id,"category":item.category,"item_type":item.item_type,"variants":variants})
                    );
                    n += 1;
                }
            }
            for prop in &c.props {
                if n < limit
                    && serde_json::to_string(prop)?
                        .to_lowercase()
                        .contains(&needle)
                {
                    let variants: Vec<_> = prop
                        .selections
                        .iter()
                        .filter_map(|id| c.selections.get(id))
                        .map(|s| readable_variant(s, &catalogue))
                        .collect();
                    println!(
                        "{}",
                        serde_json::json!({"kind":"prop","representation":prop.kind,"id":prop.id,"label":prop.label,"resource":prop.resource,"category":prop.category,"parts":prop.parts,"assemblies":prop.assemblies,"variants":variants})
                    );
                    n += 1;
                }
            }
            for asset in c
                .assets
                .values()
                .filter(|asset| asset.roles.iter().any(|role| role == "clothing"))
            {
                if n < limit
                    && serde_json::to_string(asset)?
                        .to_lowercase()
                        .contains(&needle)
                {
                    let variants: Vec<_> = asset
                        .selections
                        .iter()
                        .filter_map(|id| c.selections.get(id))
                        .map(|s| readable_variant(s, &catalogue))
                        .collect();
                    println!(
                        "{}",
                        serde_json::json!({"kind":"clothing","id":asset.id,
                        "resource":asset.resource,"roles":asset.roles,"variants":variants})
                    );
                    n += 1;
                }
            }
            for control in &c.controls {
                if n < limit
                    && serde_json::to_string(control)?
                        .to_lowercase()
                        .contains(&needle)
                {
                    println!(
                        "{}",
                        serde_json::json!({"kind":"control","name":control.native.name,"slot":control.native.ui_slot,"source":control.source,"locator":control.id,"choices":control.choices})
                    );
                    n += 1;
                }
            }
            for choice in &c.choices {
                let selection = choice
                    .selection
                    .as_ref()
                    .and_then(|id| c.selections.get(id));
                if n < limit
                    && serde_json::to_string(&(choice, &selection))?
                        .to_lowercase()
                        .contains(&needle)
                {
                    println!(
                        "{}",
                        serde_json::json!({"kind":"choice","label":choice.label,"locator":choice.id,"variant":selection.map(|v|readable_variant(v,&catalogue))})
                    );
                    n += 1;
                }
            }
        }
        Command::Show { catalogue, id } => {
            let c = publish::load(&catalogue)?;
            let v = if let Some(v) = c.items.iter().find(|v| {
                v.id == id || v.native_name.as_deref() == Some(&id) || v.native_id.to_string() == id
            }) {
                serde_json::to_value(v)?
            } else if let Some(v) = c
                .props
                .iter()
                .find(|v| v.id == id || v.resource.to_string() == id)
            {
                serde_json::to_value(v)?
            } else if let Some(v) = c.controls.iter().find(|v| v.id == id) {
                serde_json::to_value(v)?
            } else if let Some(v) = c.choices.iter().find(|v| v.id == id) {
                serde_json::to_value(v)?
            } else if let Some(v) = c.selections.get(&id) {
                serde_json::to_value(v)?
            } else if let Some(v) = c.assets.get(&id) {
                serde_json::to_value(v)?
            } else {
                anyhow::bail!("unknown catalogue ID {id}");
            };
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        Command::Render {
            catalogue,
            game,
            selection,
            work,
            size,
            cancel_file,
            reference_resource,
            reference_appearance,
            neutral_head,
        } => {
            publish::outside_game(&catalogue, &game.game)?;
            publish::outside_game(&work, &game.game)?;
            let mut c = publish::load(&catalogue)?;
            let selected = ids(&c, selection)?;
            let source = GameSource::open(&game.game, game.schema.as_deref())?;
            let references = if let Some(resource) = reference_resource {
                ids(
                    &c,
                    Selection {
                        resource: Some(resource),
                        appearance: reference_appearance,
                        item: None,
                        all: false,
                    },
                )?
            } else {
                Vec::new()
            };
            let report = preview::render(
                &mut c,
                &source,
                &selected,
                &catalogue,
                &work,
                preview::PreviewRequest {
                    framing_references: &[],
                    subject_side: None,
                    native_frame: None,
                    mesh_lods: &std::collections::BTreeMap::new(),
                    native_assembly: false,
                    origin_anchors: &std::collections::BTreeSet::new(),
                    workers: 1,
                    size,
                    cancel: cancel_file.as_deref(),
                    references: &references,
                    neutral_head,
                    neutral_assemblies: &[],
                    views: &[
                        ghostline_renderer::View::Hero,
                        ghostline_renderer::View::Front,
                        ghostline_renderer::View::Back,
                    ],
                    subject_frame: false,
                    subject_part: None,
                    subject_min_radius: None,
                    material_revision: None,
                    visibility_parameters: &std::collections::BTreeMap::from([(
                        "visibleWhenAiming".into(),
                        false,
                    )]),
                    checkpoint_clock: None,
                    subject_crop: None,
                },
            )?;
            ensure!(
                !report.outcomes.iter().any(|r| r.state == "failed"),
                "render failures are published in the catalogue"
            );
        }
        Command::PlanRenders {
            catalogue,
            out,
            family,
            size,
        } => {
            let c = publish::load(&catalogue)?;
            let plan = bulk::plan(&c, family, size)?;
            publish::atomic(&out, &serde_json::to_vec_pretty(&plan)?)?;
            println!(
                "{} jobs; {} native binding failures; plan={}",
                plan.jobs.len(),
                plan.binding_failures.len(),
                out.display()
            );
        }
        Command::RenderBatch {
            workers,
            catalogue,
            game,
            plan,
            work,
            offset,
            resume_from,
            limit,
            cancel_file,
        } => {
            publish::outside_game(&catalogue, &game.game)?;
            publish::outside_game(&work, &game.game)?;
            let mut c = publish::load(&catalogue)?;
            let plan: bulk::RenderPlan = serde_json::from_slice(&fs::read(plan)?)?;
            let source = GameSource::open(&game.game, game.schema.as_deref())?;
            for cache in resume_from {
                publish::outside_game(&cache, &game.game)?;
                preview::restore_cached(&mut c, &catalogue, &cache)?;
            }
            let end = limit.map_or(plan.jobs.len(), |limit| {
                offset.saturating_add(limit).min(plan.jobs.len())
            });
            let report = bulk::run(
                &mut c,
                &source,
                &plan,
                &catalogue,
                &work,
                offset..end,
                bulk::RunOptions {
                    workers,
                    cancel: cancel_file.as_deref(),
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            ensure!(
                !report.outcomes.iter().any(|v| v.state == "failed"),
                "render failures are listed in work/bulk-report.json and catalogue details"
            );
        }
        Command::RefreshUi { catalogue } => publish::refresh_ui(&catalogue)?,
        Command::Verify { catalogue } => {
            publish::verify(&catalogue)?;
            println!("Verified catalogue");
        }
        Command::Package {
            catalogue,
            out,
            media_base,
        } => {
            ensure!(
                media_base.is_empty()
                    || media_base.starts_with("https://")
                    || media_base.starts_with("http://"),
                "media base must be empty or an HTTP(S) URL"
            );
            let mut c = publish::load(&catalogue)?;
            publish::validate(&c, &catalogue)?;
            publish::initialize(&out)?;
            for s in c.selections.values() {
                for media in &s.media {
                    publish::atomic(
                        &out.join(&media.path),
                        &fs::read(catalogue.join(&media.path))?,
                    )?;
                }
            }
            publish::write(&mut c, &out, &media_base)?;
        }
    }
    Ok(())
}

fn ids(c: &ghostline_items::model::Catalogue, s: Selection) -> Result<Vec<String>> {
    ensure!(
        !(s.all && (s.item.is_some() || s.resource.is_some() || s.appearance.is_some())),
        "--all cannot be combined with a named selection"
    );
    ensure!(
        !(s.item.is_some() && s.resource.is_some()),
        "choose --item or --resource"
    );
    if s.all {
        return Ok(c.selections.keys().cloned().collect());
    }
    let candidates: Vec<_> = if let Some(item) = s.item {
        let item = c
            .items
            .iter()
            .find(|i| i.native_name.as_deref() == Some(&item) || i.native_id.to_string() == item)
            .with_context(|| format!("unknown native item {item}; use search to list names"))?;
        item.selections
            .iter()
            .filter_map(|id| c.selections.get(id))
            .collect()
    } else if let Some(resource) = s.resource {
        let key = ghostline_red::assets::ResourceKey::parse(&resource);
        c.selections
            .values()
            .filter(|v| v.resource.native_hash() == key.native_hash())
            .collect()
    } else {
        anyhow::bail!("select --item, --resource or --all");
    };
    let matches: Vec<_> = candidates
        .into_iter()
        .filter(|v| {
            s.appearance
                .as_deref()
                .is_none_or(|a| v.appearance.as_deref() == Some(a))
        })
        .collect();
    ensure!(
        !matches.is_empty(),
        "no matching native selection; use search/show to list authored resource paths and appearances"
    );
    if matches.len() > 1 {
        anyhow::bail!(
            "multiple native appearances; specify --appearance:\n{}",
            matches
                .iter()
                .map(|v| format!(
                    "  {} / {}",
                    v.resource,
                    v.appearance.as_deref().unwrap_or("<simple entity>")
                ))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    Ok(vec![matches[0].id.clone()])
}

fn readable_variant(
    v: &ghostline_items::model::Selection,
    catalogue: &std::path::Path,
) -> serde_json::Value {
    let mut args = vec![
        "render".to_owned(),
        "--catalogue".into(),
        catalogue.to_string_lossy().into_owned(),
        "--resource".into(),
        v.resource.to_string(),
    ];
    if let Some(appearance) = &v.appearance {
        args.extend(["--appearance".into(), appearance.clone()]);
    }
    serde_json::json!({"resource":v.resource.to_string(),"appearance":v.appearance,"render_args":args})
}
