//! Resumable preview generation for catalogues with completed native bindings.

use crate::{
    model::{Catalogue, Media},
    publish,
};
use anyhow::{Context, Result, ensure};
use ghostline_red::assets::ResourceKey;
use ghostline_renderer::{
    BatchJob, BatchLimits, BatchSource, Camera, Cancellation, GameSource, RenderOptions, Renderer,
    View,
};
use serde_json::json;
use std::{fs, path::Path};

/// Explicit native base-head frame used only as neutral preview context.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum NeutralHead {
    /// Unmorphed female player base head.
    Female,
    /// Unmorphed male player base head.
    Male,
}

impl NeutralHead {
    /// Native geometry identity; never inferred from the hairstyle filename.
    pub fn resource(self) -> ResourceKey {
        ResourceKey::Path(match self {
            Self::Female => r"base\characters\head\player_base_heads\player_female_average\h0_000_pwa__morphs.morphtarget",
            Self::Male => r"base\characters\head\player_base_heads\player_man_average\h0_000_pma__morphs.morphtarget",
        }.into())
    }

    fn label(self) -> &'static str {
        match self {
            Self::Female => "female",
            Self::Male => "male",
        }
    }
}

/// Explicit camera/output and reference-assembly recipe for a preview batch.
#[derive(Debug)]
pub struct PreviewRequest<'a> {
    /// Context selections included in camera bounds as well as drawn geometry.
    pub framing_references: &'a [String],
    /// Frame the positive or negative native X half of a paired detail.
    pub subject_side: Option<bool>,
    /// Native bone orientation for isolated rigid attachments.
    pub native_frame: Option<ghostline_renderer::NativeBoneFrame>,
    /// Explicit native mesh LOD masks for this preview recipe.
    pub mesh_lods: &'a std::collections::BTreeMap<String, u64>,
    /// Evaluate native rig and attachment bindings for this recipe.
    pub native_assembly: bool,
    /// Explicit external placement frames for a standalone native assembly.
    pub origin_anchors: &'a std::collections::BTreeSet<String>,
    /// Concurrent GPU workers; each owns its renderer and cache receipts.
    pub workers: usize,
    /// Square output dimensions.
    pub size: u32,
    /// Cooperative cancellation marker.
    pub cancel: Option<&'a Path>,
    /// Exact catalogue selections shown as reference geometry.
    pub references: &'a [String],
    /// Optional matte grey native base head, separate from the selected hair.
    pub neutral_head: Option<NeutralHead>,
    /// Exact native selections displayed with neutral materials for body context.
    pub neutral_assemblies: &'a [String],
    /// Semantic views requested for this recipe.
    pub views: &'a [View],
    /// Fit the selected asset rather than the larger reference mannequin.
    pub subject_frame: bool,
    /// Focus a single native mesh instance for details such as nails or iris colour.
    pub subject_part: Option<usize>,
    /// Minimum anatomical context around a mounted detail.
    pub subject_min_radius: Option<f32>,
    /// Recipe-scoped material correction revision.
    pub material_revision: Option<u32>,
    /// Explicit native visibility state used for catalogue previews.
    pub visibility_parameters: &'a std::collections::BTreeMap<String, bool>,
    /// Shared bulk-publication clock; individual job receipts remain immediate.
    pub checkpoint_clock: Option<&'a std::cell::Cell<std::time::Instant>>,
    /// Normalized lower/upper subject-height bounds used to frame details.
    pub subject_crop: Option<[f32; 2]>,
}

/// One reported native render job, including resumed output or a concrete failure.
#[derive(Debug, serde::Serialize)]
pub struct RenderOutcome {
    /// Native selection identity.
    pub selection: String,
    /// ready, failed or pending when cancellation interrupted the job.
    pub state: String,
    /// Native rendering failure, when present.
    pub error: Option<String>,
    /// Frames newly written by the native renderer.
    pub written_frames: usize,
    /// Frames validated and reused from native resume receipts.
    pub reused_frames: usize,
}

/// Completed work in a bounded, resumable render request.
#[derive(Debug, serde::Serialize)]
pub struct RenderSummary {
    /// Reported jobs in processing order.
    pub outcomes: Vec<RenderOutcome>,
    /// Whether cooperative cancellation stopped the request.
    pub cancelled: bool,
}

/// Renders selected isolated resources, retaining native batch receipts for resume.
///
/// Output is checkpointed after each job. A cancellation file stops at a work boundary.
pub fn render(
    c: &mut Catalogue,
    source: &GameSource,
    ids: &[String],
    package: &Path,
    work: &Path,
    request: PreviewRequest<'_>,
) -> Result<RenderSummary> {
    let size = request.size;
    let cancel = request.cancel;
    ensure!((64..=4096).contains(&size), "size must be 64..4096");
    let pending = ids
        .iter()
        .filter(|id| {
            c.selections
                .get(*id)
                .is_some_and(|s| s.resolution == "pending")
        })
        .count();
    ensure!(
        pending == 0,
        "{pending} selections lack build-time bindings; rebuild the catalogue before rendering"
    );
    let references = request
        .references
        .iter()
        .map(|id| {
            let s = c
                .selections
                .get(id)
                .with_context(|| format!("unknown reference selection {id}"))?;
            ensure!(
                s.resolution == "resolved",
                "reference bindings are not resolved: {id}"
            );
            Ok(ghostline_renderer::AssemblyReference {
                frame: request.framing_references.contains(id),
                resource: s.resource.clone(),
                appearance: s.appearance.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let neutral_assemblies = request
        .neutral_assemblies
        .iter()
        .map(|id| {
            let s = c
                .selections
                .get(id)
                .with_context(|| format!("unknown neutral reference {id}"))?;
            ensure!(
                s.resolution == "resolved",
                "neutral reference bindings are not resolved: {id}"
            );
            Ok(ghostline_renderer::AssemblyReference {
                frame: false,
                resource: s.resource.clone(),
                appearance: s.appearance.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let jobs = ids
        .iter()
        .filter(|id| {
            c.selections
                .get(*id)
                .is_none_or(|s| s.resolution != "blocked")
        })
        .map(|id| {
            let s = c
                .selections
                .get(id)
                .with_context(|| format!("unknown selection {id}"))?;
            Ok(BatchJob {
                native_frame: request.native_frame.clone(),
                name: id.clone(),
                source: if s.context["prop_kind"] == "mesh" || s.context["geometry_kind"] == "mesh"
                {
                    BatchSource::Mesh {
                        resource: s.resource.clone(),
                        appearance: s
                            .appearance
                            .clone()
                            .context("mesh selection has no appearance")?,
                    }
                } else {
                    BatchSource::Assembly {
                        resource: s.resource.clone(),
                        appearance: s.context["assembly_appearance"]
                            .as_str()
                            .map(str::to_owned)
                            .or_else(|| s.appearance.clone()),
                        options: Box::new(ghostline_renderer::AssemblyOptions {
                            mesh_lods: request.mesh_lods.clone(),
                            resource_overrides: serde_json::from_value(
                                s.context["resource_overrides"].clone(),
                            )
                            .unwrap_or_default(),
                            visibility_parameters: request.visibility_parameters.clone(),
                            bind_pose: !request.native_assembly,
                            standalone_bind_pose_fallback: request.native_assembly,
                            origin_anchors: request.origin_anchors.clone(),
                            references: references.clone(),
                            binding_contexts: serde_json::from_value(
                                s.context["binding_contexts"].clone(),
                            )
                            .unwrap_or_default(),
                            slot_bone_overrides: serde_json::from_value(
                                s.context["slot_bone_overrides"].clone(),
                            )
                            .unwrap_or_default(),
                            neutral_references: request
                                .neutral_head
                                .map(|head| vec![head.resource()])
                                .unwrap_or_default(),
                            neutral_assemblies: neutral_assemblies.clone(),
                            ..Default::default()
                        }),
                    }
                },
                views: request
                    .views
                    .iter()
                    .copied()
                    .map(|view| RenderOptions {
                        size,
                        camera: if let Some(positive) = request.subject_side {
                            Camera::FrameSubjectSide { view, positive }
                        } else if (s.context["prop"].is_string()
                            || s.context["clothing"] == true
                            || s.resource.to_string().contains("\\weapons\\"))
                            && request.subject_part.is_none()
                            && request.subject_crop.is_none()
                        {
                            Camera::FrameProjected(view)
                        } else if let Some(vertical_range) = request.subject_crop {
                            Camera::FrameSubjectCrop {
                                view,
                                part: request.subject_part,
                                vertical_range,
                            }
                        } else if let Some(part) = request.subject_part {
                            Camera::FrameSubjectPart { view, part }
                        } else if let Some(min_radius) = request.subject_min_radius {
                            Camera::FrameSubjectRegion { view, min_radius }
                        } else if request.subject_frame {
                            Camera::FrameSubject(view)
                        } else {
                            Camera::Frame(view)
                        },
                    })
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let skipped = ids.len() - jobs.len();
    if skipped > 0 {
        eprintln!(
            "Skipping {skipped} selections with blocked build-time bindings; see data/binding-report.json"
        );
    }
    ensure!(
        !jobs.is_empty(),
        "no resolved selections to render; inspect data/binding-report.json"
    );
    ensure!(request.workers > 0, "workers must be greater than zero");
    let cancellation = cancel.map_or_else(Cancellation::default, |p| {
        Cancellation::with_file(p.to_owned())
    });
    let mut error = None;
    let mut outcomes = Vec::new();
    let mut ready_progress = 0;
    let mut failed_progress = 0;
    let media_base = fs::read(package.join("manifest.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| v["media_base"].as_str().map(str::to_owned))
        .unwrap_or_default();
    let mut failed = 0;
    let mut reported = 0;
    // Bulk rendering starts from a built catalogue and validates each new
    // image at commit. Do not rescan every published image at the first timed
    // checkpoint of every recipe group.
    let mut checkpoint_validated = request.checkpoint_clock.is_some();
    std::thread::scope(|scope| -> Result<()> {
        let worker_count = request.workers.min(jobs.len());
        // Results contain receipts/paths, not frame buffers. Keep a bounded
        // backlog so metadata publication can run without parking every GPU
        // worker after just one additional result each.
        let backlog = worker_count.saturating_mul(64).min(jobs.len());
        let (sender, receiver) = std::sync::mpsc::sync_channel(backlog);
        for worker in 0..worker_count {
            let sender = sender.clone();
            let cancellation = cancellation.clone();
            let assigned: Vec<_> = jobs
                .iter()
                .skip(worker)
                .step_by(worker_count)
                .cloned()
                .collect();
            scope.spawn(move || {
                let run = || -> Result<()> {
                    let renderer = pollster::block_on(Renderer::new())?;
                    eprintln!("GPU worker {worker}: {:?}", renderer.adapter_info());
                    let prefix = format!("workers/{worker}");
                    let worker_work = work.join(&prefix);
                    for chunk in assigned.chunks(BatchLimits::default().jobs) {
                        renderer.run_batch(
                            source,
                            chunk,
                            &worker_work,
                            BatchLimits::default(),
                            &cancellation,
                            |result| {
                                let mut result = result.clone();
                                result.directory = format!("{prefix}/{}", result.directory);
                                if sender.send(Ok(result)).is_err() {
                                    cancellation.cancel();
                                }
                            },
                        )?;
                        if cancellation.is_cancelled() {
                            break;
                        }
                    }
                    Ok(())
                };
                if let Err(error) = run() {
                    cancellation.cancel();
                    let _ = sender.send(Err(error));
                }
            });
        }
        drop(sender);
        for message in receiver {
            match message {
                Err(e) => {
                    error = Some(e);
                    cancellation.cancel();
                }
                Ok(result) => {
                    eprintln!(
                        "{}: {}",
                        result.name,
                        result.error.as_deref().unwrap_or(if result.non_visual {
                            "nonvisual native assembly"
                        } else {
                            "complete"
                        })
                    );
                    if let Err(e) = commit_result(c, &result, package, work, &request) {
                        error = Some(e);
                        cancellation.cancel();
                    } else {
                        outcomes.push(RenderOutcome {
                            selection: result.name.clone(),
                            state: if result.non_visual {
                                "not_applicable"
                            } else if result.error.is_some() {
                                "failed"
                            } else if result.output.as_ref().is_some_and(|r| !r.cancelled) {
                                "ready"
                            } else {
                                "pending"
                            }
                            .into(),
                            error: result.error.clone(),
                            written_frames: result.output.as_ref().map_or(0, |r| r.written),
                            reused_frames: result.output.as_ref().map_or(0, |r| r.reused),
                        });
                        ready_progress +=
                            usize::from(outcomes.last().is_some_and(|v| v.state == "ready"));
                        failed_progress +=
                            usize::from(outcomes.last().is_some_and(|v| v.state == "failed"));
                        if let Err(e)=publish::atomic(&work.join("render-progress.json"),&serde_json::to_vec(&json!({
                        "requested":jobs.len(),"reported":outcomes.len(),"ready":ready_progress,"failed":failed_progress,"last":outcomes.last()
                    })).expect("serializable progress")) {error=Some(e);cancellation.cancel();}
                    }
                    failed += usize::from(result.error.is_some());
                    reported += 1;
                    let checkpoint_due = request
                        .checkpoint_clock
                        .map_or(reported % BatchLimits::default().jobs == 0, |clock| {
                            clock.get().elapsed() >= std::time::Duration::from_secs(120)
                        });
                    if checkpoint_due && error.is_none() {
                        let snapshot = if checkpoint_validated {
                            publish::checkpoint(c, package, &media_base)
                        } else {
                            publish::write(c, package, &media_base)
                        };
                        match snapshot {
                            Ok(()) => checkpoint_validated = true,
                            Err(e) => {
                                error = Some(e);
                                cancellation.cancel();
                            }
                        }
                    }
                    if checkpoint_due
                        && error.is_none()
                        && let Some(clock) = request.checkpoint_clock
                    {
                        clock.set(std::time::Instant::now());
                    }
                }
            }
        }
        Ok(())
    })?;
    if request.checkpoint_clock.is_none() || error.is_some() || cancellation.is_cancelled() {
        if request.checkpoint_clock.is_some() {
            publish::checkpoint(c, package, &media_base)?;
        } else {
            publish::write(c, package, &media_base)?;
        }
        // Bulk rendering only adds media after its initial invalidation pass.
        // Prune once at bulk completion, not on every checkpoint/cancellation.
        if request.checkpoint_clock.is_none() {
            publish::prune_media(c, package)?;
        }
    }
    if let Some(e) = error {
        return Err(e);
    }
    eprintln!(
        "{reported} jobs reported; {failed} failed; cancelled={}",
        cancellation.is_cancelled()
    );
    Ok(RenderSummary {
        outcomes,
        cancelled: cancellation.is_cancelled(),
    })
}

/// Compares the native inputs consumed by isolated bind-pose previews.
/// Runtime skin/control bindings are deliberately not evaluated in this mode.
pub fn same_bind_pose_inputs(a: &crate::model::Selection, b: &crate::model::Selection) -> bool {
    // Older receipts did not publish the decoder's shadow-only classification.
    // The current verified classification applies to both snapshots; those
    // meshes were already omitted by the renderer and never affect this preview.
    let shadows: std::collections::BTreeSet<_> = [a, b]
        .into_iter()
        .flat_map(|selection| {
            selection.context["material_bindings"]
                .as_array()
                .into_iter()
                .flatten()
        })
        .filter(|binding| binding["binding"]["shadow_only"] == true)
        .filter_map(|binding| binding["component"].as_str().map(str::to_owned))
        .collect();
    let resource = |value: &mut serde_json::Value| {
        if let Ok(key) = serde_json::from_value::<ResourceKey>(value.clone()) {
            *value = json!({"Hash":key.native_hash().to_string()});
        }
    };
    let components = |selection: &crate::model::Selection| {
        let native: std::collections::BTreeMap<_, _> = selection
            .components
            .iter()
            .filter_map(|component| Some((component["name"].as_str()?, component)))
            .collect();
        let mut pending: Vec<_> = native
            .iter()
            .filter(|(name, component)| {
                !component["mesh"].is_null()
                    && component["enabled"] != false
                    && component["chunk_mask"] != "0"
                    && !shadows.contains(**name)
            })
            .map(|(name, _)| *name)
            .collect();
        let mut graph = std::collections::BTreeMap::new();
        while let Some(name) = pending.pop() {
            if graph.contains_key(name) {
                continue;
            }
            if let Some(source) = native.get(name) {
                let mut component = (*source).clone();
                if let Some(object) = component.as_object_mut() {
                    for key in ["control", "rig", "slots", "mesh_appearance_requested"] {
                        object.remove(key);
                    }
                }
                resource(&mut component["mesh"]);
                let parent = source["parent"]["component"].as_str().unwrap_or_default();
                let slot = source["parent"]["slot"].as_str().unwrap_or_default();
                if native.contains_key(parent)
                    && (slot.is_empty() || slot.eq_ignore_ascii_case("none"))
                {
                    pending.push(parent);
                    component["parent"] = json!({"component":parent});
                } else {
                    component["parent"] = serde_json::Value::Null;
                }
                graph.insert(name.to_owned(), component);
            }
        }
        graph
    };
    let materials = |selection: &crate::model::Selection| {
        let mut bindings = selection.context["material_bindings"].clone();
        if let Some(bindings) = bindings.as_array_mut() {
            bindings.retain(|binding| {
                binding["component"]
                    .as_str()
                    .is_none_or(|name| !shadows.contains(name))
            });
        }
        for binding in bindings.as_array_mut().into_iter().flatten() {
            if binding["binding"].get("shadow_only").is_none() {
                binding["binding"]["shadow_only"] = json!(false);
            }
            if binding["binding"].get("non_visual").is_none() {
                binding["binding"]["non_visual"] = json!(false);
            }
            resource(&mut binding["binding"]["geometry"]);
            resource(&mut binding["binding"]["material_source"]);
            if let Some(object) = binding["binding"].as_object_mut() {
                object.remove("requested_appearance");
                object.remove("used_native_default");
            }
        }
        bindings
    };
    a.resource == b.resource
        && ghostline_red::assets::same_native_value(
            a.context.get("binding_contexts").unwrap_or(&json!([])),
            b.context.get("binding_contexts").unwrap_or(&json!([])),
        )
        && ghostline_red::assets::same_native_value(
            a.context.get("slot_bone_overrides").unwrap_or(&json!({})),
            b.context.get("slot_bone_overrides").unwrap_or(&json!({})),
        )
        && ghostline_red::assets::same_native_value(
            &a.context["prop_components"],
            &b.context["prop_components"],
        )
        && a.appearance == b.appearance
        && a.resolution == "resolved"
        && b.resolution == "resolved"
        && ghostline_red::assets::same_native_value(&json!(components(a)), &json!(components(b)))
        && ghostline_red::assets::same_native_value(&materials(a), &materials(b))
}

/// Restores completed preview publication from an explicitly selected same-source
/// render cache. Native geometry/material bindings and every saved frame are checked;
/// normal bulk recipe validation still decides whether the preview can be reused.
///
/// # Errors
/// Returns an error for malformed receipts or inaccessible cache/output files.
pub fn restore_cached(c: &mut Catalogue, package: &Path, cache: &Path) -> Result<usize> {
    let workers = cache.join("workers");
    if !cache.is_dir() {
        return Ok(0);
    }
    let mut parents = vec![cache.to_path_buf()];
    if workers.is_dir() {
        for worker in fs::read_dir(workers)? {
            let worker = worker?.path();
            if worker.is_dir() {
                parents.push(worker);
            }
        }
    }
    let mut receipts = Vec::new();
    for parent in parents {
        for job in fs::read_dir(parent)? {
            let directory = job?.path();
            let receipt = directory.join("catalogue-selection.json");
            if receipt.is_file() {
                receipts.push((fs::metadata(&receipt)?.modified()?, directory));
            }
        }
    }
    receipts.sort_by_key(|receipt| std::cmp::Reverse(receipt.0));
    let mut restored = 0;
    let mut retained = 0;
    for (modified, directory) in receipts {
        let stamp = modified.duration_since(std::time::UNIX_EPOCH)?.as_nanos();
        let receipt = directory.join("catalogue-selection.json");
        let prior: crate::model::Selection = serde_json::from_slice(&fs::read(receipt)?)?;
        if prior.preview_state != "ready" || prior.media.is_empty() {
            continue;
        }
        let Some(selection) = c.selections.get_mut(&prior.id) else {
            continue;
        };
        if selection.context["render"]["cache_receipt_modified_ns"]
            .as_str()
            .and_then(|s| s.parse::<u128>().ok())
            .is_some_and(|current| current >= stamp)
        {
            continue;
        }
        if !same_bind_pose_inputs(selection, &prior) {
            continue;
        }
        // A completed catalogue already contains these immutable images. Resume
        // must not copy tens of gigabytes over valid published files again.
        let published = selection.preview_state == "ready"
            && selection.media.len() == prior.media.len()
            && selection
                .media
                .iter()
                .zip(&prior.media)
                .all(|(current, saved)| {
                    let path = Path::new(&current.path);
                    current.path == saved.path
                        && current.bytes == saved.bytes
                        && current.size == saved.size
                        && current.view == saved.view
                        && !path.is_absolute()
                        && path
                            .components()
                            .all(|part| matches!(part, std::path::Component::Normal(_)))
                        && fs::metadata(package.join(path))
                            .is_ok_and(|file| file.is_file() && file.len() == current.bytes)
                });
        if published {
            selection.context["render"]["cache_receipt_modified_ns"] = json!(stamp.to_string());
            retained += 1;
            continue;
        }
        if !prior.media.iter().enumerate().all(|(i, media)| {
            fs::metadata(directory.join(format!("images/{i:06}.png")))
                .is_ok_and(|file| file.is_file() && file.len() == media.bytes)
        }) {
            continue;
        }
        for (i, media) in prior.media.iter().enumerate() {
            let member = std::path::Path::new(&media.path);
            ensure!(
                !member.is_absolute()
                    && member
                        .components()
                        .all(|part| matches!(part, std::path::Component::Normal(_))),
                "unsafe cached media path"
            );
            let target = package.join(member);
            fs::create_dir_all(target.parent().context("cached media parent")?)?;
            // The catalogue is published only after this restoration pass.
            // Copy immutable completed PNGs directly; per-frame fsync/rename
            // adds thousands of serial disk flushes without exposing readers
            // to an incomplete catalogue entry.
            fs::copy(directory.join(format!("images/{i:06}.png")), target)?;
        }
        selection.media = prior.media;
        selection.preview_state = "ready".into();
        selection.context["render"] = prior.context["render"].clone();
        selection.context["render"]["cache_receipt_modified_ns"] = json!(stamp.to_string());
        restored += 1;
    }
    eprintln!(
        "Retained {retained} published previews; restored {restored} from {}",
        cache.display()
    );
    Ok(restored)
}

fn commit_result(
    c: &mut Catalogue,
    result: &ghostline_renderer::JobResult,
    package: &Path,
    work: &Path,
    request: &PreviewRequest<'_>,
) -> Result<()> {
    let size = request.size;
    let references = request.references;
    let selection = c
        .selections
        .get_mut(&result.name)
        .context("batch returned unknown selection")?;
    selection.diagnostics.retain(|d| !d.starts_with("render: "));
    if result.non_visual {
        selection.preview_state = "not_applicable".into();
        selection.media.clear();
        selection.context["preview_omission"] =
            json!("Native assembly contains no enabled visible geometry");
    } else if let Some(reason) = &result.error {
        selection.preview_state = "failed".into();
        selection.media.clear();
        selection.diagnostics.push(format!("render: {reason}"));
    } else if result.output.as_ref().is_some_and(|r| !r.cancelled) {
        let key = selection.resource.native_hash();
        let identity = portable_name(selection.appearance.as_deref().unwrap_or("default"));
        let slug = match &selection.resource {
            ResourceKey::Path(p) => portable_name(p.rsplit(['\\', '/']).next().unwrap_or("asset")),
            ResourceKey::Hash(_) => "asset".into(),
        };
        let mut media = Vec::new();
        for (index, view) in request
            .views
            .iter()
            .map(|v| match v {
                View::Hero => "hero",
                View::Front => "front",
                View::Back => "back",
                View::Left => "left",
                View::Right => "right",
                View::Side => "side",
            })
            .enumerate()
        {
            let bytes = fs::read(
                work.join(&result.directory)
                    .join(format!("images/{index:06}.png")),
            )?;
            let context = request
                .neutral_head
                .map(|h| format!("neutral-head-{}/", h.label()))
                .unwrap_or_default();
            let path =
                format!("media/images/{slug}--{key}/{identity}/{context}{size}px/{view}.png");
            publish::atomic(&package.join(&path), &bytes)?;
            media.push(Media {
                path,
                view: view.into(),
                bytes: bytes.len() as u64,
                size,
            });
        }
        selection.media = media;
        selection.preview_state = "ready".into();
        selection.context["render"] = json!({"kind":"bind_pose","references":references,"neutral_head":request.neutral_head.map(|h|json!({"frame":h,"resource":h.resource(),"material":"matte_grey","morph_weights":"baseline"})),"neutral_assemblies":request.neutral_assemblies,"subject_frame":request.subject_frame,"subject_part":request.subject_part,"subject_min_radius":request.subject_min_radius,"material_revision":request.material_revision,"external_bindings_evaluated":false,"size":size,"views":request.views,"approximation":"Native preview materials; compare with game for fidelity. Reference geometry is preview context only; native rig fit is not evaluated."});
    }
    if selection.preview_state == "ready" {
        selection.context["render"]["native_frame"] = json!(request.native_frame);
        selection.context["render"]["subject_side"] = json!(request.subject_side);
        selection.context["render"]["framing_references"] = json!(request.framing_references);
        selection.context["render"]["mesh_lods"] = json!(request.mesh_lods);
        selection.context["render"]["native_assembly"] = json!(request.native_assembly);
        selection.context["render"]["origin_anchors"] = json!(request.origin_anchors);
        if request.native_assembly {
            selection.context["render"]["kind"] = json!("native_assembly");
            selection.context["render"]["external_bindings_evaluated"] = json!(true);
            selection.context["render"]["approximation"] = json!(
                "Native bind-pose rig and attachment evaluation; explicit external placement frames frozen at origin; native preview materials"
            );
        }
        selection.context["render"]["subject_crop"] = json!(request.subject_crop);
        selection.context["render"]["visibility_parameters"] = json!(request.visibility_parameters);
        let inspection_path = work
            .join(&result.directory)
            .join("assembly-inspection.json");
        if inspection_path.is_file() {
            let inspection: serde_json::Value =
                serde_json::from_slice(&fs::read(inspection_path)?)?;
            let notes: std::collections::BTreeSet<_> = inspection["assets"]
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|asset| asset["materials"].as_array().into_iter().flatten())
                .flat_map(|material| material["notes"].as_array().into_iter().flatten())
                .filter_map(serde_json::Value::as_str)
                .chain(
                    inspection["preview_notes"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(serde_json::Value::as_str),
                )
                .collect();
            selection.context["render"]["material_notes"] = json!(notes);
        }
    }
    publish::atomic(
        &work
            .join(&result.directory)
            .join("catalogue-selection.json"),
        &serde_json::to_vec(selection)?,
    )?;
    Ok(())
}

fn portable_name(value: &str) -> String {
    use std::fmt::Write as _;
    let mut name = String::new();
    for byte in value.as_bytes() {
        if byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_') {
            name.push(char::from(*byte));
        } else {
            write!(name, "~{byte:02X}").expect("write to string");
        }
    }
    let reserved = matches!(
        name.to_ascii_uppercase().as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );
    if reserved {
        name = format!("~{:02X}{}", name.as_bytes()[0], &name[1..]);
    }
    if name.is_empty() {
        name = "~empty".into();
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bind_pose_retention_checks_visible_inputs_without_runtime_controls() {
        let make = || {
            serde_json::from_value::<crate::model::Selection>(json!({
            "id":"part#default","resource":{"Hash":"42"},"appearance":"default",
            "context":{"material_bindings":[{"binding":{"appearance":"default"}}]},
            "components":[{"name":"part","mesh":{"Hash":"43"},"control":{"component":""},"local_transform":[0.09999847412109376]}],
            "resolution":"resolved","preview_state":"ready","diagnostics":[],"media":[]
        })).unwrap()
        };
        let mut prior = make();
        let mut current = make();
        current.components[0]["control"]["component"] = json!("runtime_rig");
        current.components[0]["local_transform"][0] = json!(0.09999847412109375);
        current.context["material_bindings"][0]["binding"]["shadow_only"] = json!(false);
        assert!(same_bind_pose_inputs(&prior, &current));
        let shadow = json!({"name":"shadow","mesh":{"Hash":"45"},"control":{"component":""}});
        prior.components.push(shadow.clone());
        current.components.push(shadow);
        prior.context["material_bindings"]
            .as_array_mut()
            .unwrap()
            .push(json!({"component":"shadow","binding":{"geometry":{"Hash":"45"}}}));
        current.context["material_bindings"].as_array_mut().unwrap().push(json!({"component":"shadow","binding":{"geometry":{"Hash":"45"},"shadow_only":true}}));
        assert!(same_bind_pose_inputs(&prior, &current));
        current.components[0]["mesh"] = json!({"Hash":"44"});
        assert!(!same_bind_pose_inputs(&prior, &current));
    }

    #[test]
    fn native_appearance_punctuation_does_not_collide_in_media_paths() {
        let names = [
            "look&a", "look-a", "look~26a", "CON", "~43ON", "", "look", "LOOK",
        ];
        let paths: std::collections::BTreeSet<_> = names.iter().map(|v| portable_name(v)).collect();
        assert_eq!(paths.len(), names.len());
        assert_eq!(portable_name("look&a"), "look~26a");
    }
}
