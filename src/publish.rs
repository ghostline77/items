//! Self-contained static packages and validated machine-readable catalogue manifests.

use crate::model::Catalogue;
mod browser;
use anyhow::{Context, Result, ensure};
use serde_json::json;
use std::{
    fs,
    io::Write,
    path::{Component, Path},
};

/// Removes generated media no longer referenced by the published catalogue.
/// The resolved media directory must remain inside the owned package root.
///
/// # Errors
/// Returns an error for symlinks, unsafe output paths or filesystem failures.
pub fn prune_media(c: &Catalogue, root: &Path) -> Result<usize> {
    let root = fs::canonicalize(root)?;
    let media = root.join("media");
    if !media.exists() {
        return Ok(0);
    }
    ensure!(
        !fs::symlink_metadata(&media)?.file_type().is_symlink(),
        "media directory cannot be a symlink"
    );
    ensure!(
        fs::canonicalize(&media)?.starts_with(&root),
        "media escapes package root"
    );
    let used: std::collections::BTreeSet<_> = c
        .selections
        .values()
        .flat_map(|s| {
            s.media.iter().map(|m| {
                let path = m.path.replace('\\', "/");
                if cfg!(windows) {
                    path.to_lowercase()
                } else {
                    path
                }
            })
        })
        .collect();
    let mut directories = vec![media.clone()];
    let mut visited = Vec::new();
    let mut removed = 0;
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            let path = entry.path();
            ensure!(!kind.is_symlink(), "media entries cannot be symlinks");
            ensure!(
                fs::canonicalize(&path)?.starts_with(&media),
                "media entry escapes media directory"
            );
            if kind.is_dir() {
                directories.push(path);
            } else if kind.is_file() {
                let relative = path
                    .strip_prefix(&root)?
                    .to_string_lossy()
                    .replace('\\', "/");
                let comparison = if cfg!(windows) {
                    relative.to_lowercase()
                } else {
                    relative
                };
                if !used.contains(comparison.as_str()) {
                    #[cfg(windows)]
                    {
                        let mut permissions = fs::metadata(&path)?.permissions();
                        if permissions.readonly() {
                            #[expect(
                                clippy::permissions_set_readonly_false,
                                reason = "Windows-only generated media cleanup; this clears FILE_ATTRIBUTE_READONLY and is never compiled on Unix"
                            )]
                            permissions.set_readonly(false);
                            fs::set_permissions(&path, permissions).with_context(|| {
                                format!(
                                    "clear generated media read-only attribute {}",
                                    path.display()
                                )
                            })?;
                        }
                    }
                    for attempt in 0..8 {
                        match fs::remove_file(&path) {
                            Ok(()) => break,
                            Err(error)
                                if cfg!(windows)
                                    && attempt < 7
                                    && matches!(error.raw_os_error(), Some(5 | 32 | 33)) =>
                            {
                                std::thread::sleep(std::time::Duration::from_millis(20 << attempt));
                            }
                            Err(error) => {
                                return Err(error).with_context(|| {
                                    format!("remove obsolete media {}", path.display())
                                });
                            }
                        }
                    }
                    removed += 1;
                }
            }
        }
        visited.push(directory);
    }
    // Children are visited after their parents; remove empty leaves first so
    // obsolete appearance and size directories do not accumulate in releases.
    for directory in visited.into_iter().rev() {
        if directory != media
            && fs::read_dir(&directory)?.next().is_none()
            && let Err(error) = fs::remove_dir(&directory)
            && !matches!(
                error.kind(),
                std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::DirectoryNotEmpty
            )
        {
            return Err(error)
                .with_context(|| format!("remove empty media directory {}", directory.display()));
        }
    }
    Ok(removed)
}

/// Checks that a generated destination is outside the installed game.
///
/// # Errors
/// Returns an error for paths inside the game or inaccessible directories.
pub fn outside_game(output: &Path, game: &Path) -> Result<()> {
    let absolute = std::path::absolute(output)?;
    let output = absolute.as_path();
    let mut ancestor = output;
    let mut suffix = Vec::new();
    while !ancestor.exists() {
        suffix.push(
            ancestor
                .file_name()
                .context("destination has no existing ancestor")?,
        );
        ancestor = ancestor.parent().context("destination has no parent")?;
    }
    let mut resolved = ancestor.canonicalize()?;
    for part in suffix.into_iter().rev() {
        resolved.push(part);
    }
    ensure!(
        !resolved.starts_with(game.canonicalize()?),
        "output must be outside the game installation"
    );
    ensure!(
        !output
            .components()
            .any(|c| matches!(c, Component::ParentDir)),
        "output must not contain parent traversal"
    );
    Ok(())
}

/// Opens an owned catalogue package or creates a new empty destination.
///
/// # Errors
/// Refuses to overwrite unrelated files or an unknown package format.
pub fn initialize(root: &Path) -> Result<()> {
    fs::create_dir_all(root)?;
    let marker = root.join("manifest.json");
    if marker.exists() {
        let v: serde_json::Value = serde_json::from_slice(&fs::read(marker)?)?;
        ensure!(
            v["format"] == "ghostline-items" && v["version"] == 1,
            "unrecognized package manifest"
        );
    } else {
        ensure!(
            fs::read_dir(root)?.next().is_none(),
            "destination is not an empty directory or items package"
        );
    }
    Ok(())
}

/// Reads a catalogue descriptor and its typed collection shards.
///
/// # Errors
/// Returns an error for missing shards, unsafe paths or unsupported formats.
pub fn load(root: &Path) -> Result<Catalogue> {
    let mut descriptor: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("data/catalogue.json"))?)?;
    ensure!(
        descriptor["format"] == "ghostline-items" && descriptor["version"] == 1,
        "unsupported catalogue format"
    );
    let collections = descriptor["collections"]
        .as_object()
        .context("missing collection index")?
        .clone();
    descriptor
        .as_object_mut()
        .context("invalid descriptor")?
        .remove("collections");
    for (name, files) in collections {
        let mut entries = Vec::new();
        for file in files.as_array().context("invalid collection shard list")? {
            let path = file.as_str().context("invalid shard path")?;
            safe_relative(path)?;
            let shard: Vec<serde_json::Value> =
                serde_json::from_slice(&fs::read(root.join(path))?)?;
            entries.extend(shard);
        }
        let value = if name == "assets" || name == "selections" {
            let mut out = serde_json::Map::new();
            for entry in entries {
                let id = entry["id"].as_str().context("entry has no ID")?.to_owned();
                ensure!(out.insert(id, entry).is_none(), "duplicate shard identity");
            }
            serde_json::Value::Object(out)
        } else {
            serde_json::Value::Array(entries)
        };
        descriptor[&name] = value;
    }
    Ok(serde_json::from_value(descriptor)?)
}

fn safe_relative(path: &str) -> Result<()> {
    ensure!(
        !path.is_empty()
            && Path::new(path)
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
        "unsafe relative package path"
    );
    Ok(())
}

/// Writes bounded JSON/detail shards, a compact browser index and catalogue manifest.
///
/// # Errors
/// Returns an error for broken references, invalid media or publication I/O.
pub fn write(c: &mut Catalogue, root: &Path, media_base: &str) -> Result<()> {
    write_inner(c, root, media_base, true)
}

/// Publishes a render checkpoint after the first fully validated snapshot.
/// The coordinator only adds atomically committed media between snapshots;
/// native bindings and catalogue relationships remain fixed during rendering.
pub(crate) fn checkpoint(c: &mut Catalogue, root: &Path, media_base: &str) -> Result<()> {
    write_inner(c, root, media_base, false)
}

fn write_inner(
    c: &mut Catalogue,
    root: &Path,
    media_base: &str,
    validate_media: bool,
) -> Result<()> {
    let started = std::time::Instant::now();
    eprintln!("Publishing {} catalogue selections...", c.selections.len());
    if validate_media {
        validate(c, root)?;
        eprintln!(
            "Catalogue validation complete in {:.1}s",
            started.elapsed().as_secs_f64()
        );
    }
    c.update_coverage();
    let mut files = std::collections::BTreeMap::<String, u64>::new();
    let mut collections = serde_json::Map::new();
    let mut browser = json!({"format":c.format,"version":c.version,"coverage":c.coverage,"game_version":c.sources.get("game_version")});
    type Entries<'a> =
        Box<dyn Iterator<Item = std::result::Result<serde_json::Value, serde_json::Error>> + 'a>;
    let lists: [(&str, Entries<'_>); 6] = [
        ("props", Box::new(c.props.iter().map(serde_json::to_value))),
        ("items", Box::new(c.items.iter().map(serde_json::to_value))),
        (
            "assets",
            Box::new(c.assets.values().map(serde_json::to_value)),
        ),
        (
            "controls",
            Box::new(c.controls.iter().map(serde_json::to_value)),
        ),
        (
            "choices",
            Box::new(c.choices.iter().map(serde_json::to_value)),
        ),
        (
            "selections",
            Box::new(c.selections.values().map(serde_json::to_value)),
        ),
    ];
    for (name, mut entries) in lists {
        let collection_started = std::time::Instant::now();
        let mut paths = Vec::new();
        let mut summaries = Vec::new();
        loop {
            let chunk = entries
                .by_ref()
                .take(128)
                .collect::<std::result::Result<Vec<_>, _>>()?;
            if chunk.is_empty() {
                break;
            }
            let bytes = serde_json::to_vec(&chunk)?;
            let index = paths.len();
            let path = format!("data/{name}/{index:04}.json");
            let script = format!("data/browser/details/{name}-{index:04}.js");
            atomic(&root.join(&path), &bytes)?;
            files.insert(path.clone(), bytes.len() as u64);
            paths.push(path);
            let mut js = b"globalThis.GHOSTLINE_DETAILS??={};for(const v of ".to_vec();
            js.extend(&bytes);
            js.extend(b")globalThis.GHOSTLINE_DETAILS[v.id]=v;\n");
            atomic(&root.join(&script), &js)?;
            files.insert(script.clone(), js.len() as u64);
            for entry in &chunk {
                summaries.push(super::publish::browser::summary(entry, &script, c));
            }
        }
        eprintln!(
            "Published {name}: {} entries in {:.1}s",
            summaries.len(),
            collection_started.elapsed().as_secs_f64()
        );
        collections.insert(name.into(), serde_json::to_value(paths)?);
        browser[name] = if name == "assets" || name == "selections" {
            serde_json::Value::Object(
                summaries
                    .into_iter()
                    .map(|v| (v["id"].as_str().expect("summary ID").to_owned(), v))
                    .collect(),
            )
        } else {
            serde_json::Value::Array(summaries)
        };
    }
    let descriptor = json!({"format":c.format,"version":c.version,"sources":c.sources,"coverage":c.coverage,
        "diagnostics":c.diagnostics,"customization_metadata":c.customization_metadata,"collections":collections});
    let bytes = serde_json::to_vec(&descriptor)?;
    atomic(&root.join("data/catalogue.json"), &bytes)?;
    files.insert("data/catalogue.json".into(), bytes.len() as u64);
    let mut js = b"globalThis.GHOSTLINE_CATALOGUE=".to_vec();
    js.extend(serde_json::to_vec(&browser)?);
    js.extend(b";\n");
    atomic(&root.join("data/browser/catalogue.js"), &js)?;
    files.insert("data/browser/catalogue.js".into(), js.len() as u64);
    for (path, bytes) in [
        (
            "assets/app.js",
            include_bytes!("../web/dist/assets/app.js").as_slice(),
        ),
        (
            "assets/style.css",
            include_bytes!("../web/dist/assets/style.css").as_slice(),
        ),
        (
            "index.html",
            include_bytes!("../web/dist/index.html").as_slice(),
        ),
    ] {
        atomic(&root.join(path), bytes)?;
        files.insert(path.into(), bytes.len() as u64);
    }
    let config = format!(
        "globalThis.GHOSTLINE_MEDIA_BASE={};\n",
        serde_json::to_string(media_base)?
    );
    atomic(&root.join("assets/config.js"), config.as_bytes())?;
    files.insert("assets/config.js".into(), config.len() as u64);
    let coverage = serde_json::to_vec_pretty(&c.coverage)?;
    atomic(&root.join("data/coverage.json"), &coverage)?;
    files.insert("data/coverage.json".into(), coverage.len() as u64);
    let report = json!({"resolution_states":c.coverage["resolution_states"],
        "failures":c.selections.values().filter(|s|s.resolution=="blocked").map(|s|json!({
            "selection":s.id,"resource":s.resource,"appearance":s.appearance,
            "diagnostics":s.diagnostics.iter().filter(|d|d.starts_with("resolution: ")).collect::<Vec<_>>()
        })).collect::<Vec<_>>(),
        "item_diagnostics":c.items.iter().filter(|i|!i.diagnostics.is_empty()).map(|i|json!({"item":i.id,"native_name":i.native_name,"diagnostics":i.diagnostics})).collect::<Vec<_>>(),
        "source_diagnostics":c.diagnostics});
    let report = serde_json::to_vec_pretty(&report)?;
    atomic(&root.join("data/binding-report.json"), &report)?;
    files.insert("data/binding-report.json".into(), report.len() as u64);
    atomic(
        &root.join("manifest.json"),
        &serde_json::to_vec_pretty(
            &json!({"format":c.format,"version":c.version,"sources":c.sources,
        "catalogue":{"path":"data/catalogue.json","bytes":bytes.len() as u64},"files":files,"coverage":"data/coverage.json","media_base":media_base}),
        )?,
    )?;
    eprintln!(
        "Catalogue publication complete in {:.1}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

/// Validates the catalogue graph, file lengths, browser code and detail data.
///
/// # Errors
/// Returns an error for missing/truncated files, invalid data or broken references.
pub fn verify(root: &Path) -> Result<()> {
    let c = load(root)?;
    validate(&c, root)?;
    let m: serde_json::Value = serde_json::from_slice(&fs::read(root.join("manifest.json"))?)?;
    for (path, size) in m["files"].as_object().context("manifest has no files")? {
        safe_relative(path)?;
        ensure!(
            fs::metadata(root.join(path))?.len() == size.as_u64().context("invalid file length")?,
            "package file length mismatch: {path}"
        );
    }
    for (path, expected) in [
        (
            "assets/app.js",
            include_bytes!("../web/dist/assets/app.js").as_slice(),
        ),
        (
            "assets/style.css",
            include_bytes!("../web/dist/assets/style.css").as_slice(),
        ),
        (
            "index.html",
            include_bytes!("../web/dist/index.html").as_slice(),
        ),
    ] {
        ensure!(
            fs::read(root.join(path))? == expected,
            "browser asset differs: {path}"
        );
    }
    let descriptor: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("data/catalogue.json"))?)?;
    for (name, paths) in descriptor["collections"]
        .as_object()
        .context("missing collections")?
    {
        for (index, path) in paths
            .as_array()
            .context("missing shard paths")?
            .iter()
            .enumerate()
        {
            let path = path.as_str().context("invalid shard path")?;
            safe_relative(path)?;
            let mut expected = b"globalThis.GHOSTLINE_DETAILS??={};for(const v of ".to_vec();
            expected.extend(fs::read(root.join(path))?);
            expected.extend(b")globalThis.GHOSTLINE_DETAILS[v.id]=v;\n");
            ensure!(
                fs::read(root.join(format!("data/browser/details/{name}-{index:04}.js")))?
                    == expected,
                "browser detail data differs: {path}"
            );
        }
    }
    Ok(())
}

/// Checks all catalogue links and verifies every published ready image.
pub fn validate(c: &Catalogue, root: &Path) -> Result<()> {
    let canonical_root = root.canonicalize()?;
    let controls: std::collections::BTreeSet<_> =
        c.controls.iter().map(|v| v.id.as_str()).collect();
    let choices: std::collections::BTreeSet<_> = c.choices.iter().map(|v| v.id.as_str()).collect();
    ensure!(
        controls.len() == c.controls.len() && choices.len() == c.choices.len(),
        "duplicate control or choice ID"
    );
    for item in &c.items {
        for id in &item.selections {
            ensure!(c.selections.contains_key(id), "missing item selection {id}");
        }
    }
    let props: std::collections::BTreeSet<_> = c.props.iter().map(|p| p.id.as_str()).collect();
    ensure!(props.len() == c.props.len(), "duplicate prop ID");
    for prop in &c.props {
        for id in &prop.selections {
            ensure!(c.selections.contains_key(id), "missing prop selection {id}");
        }
        for id in prop.parts.iter().chain(&prop.assemblies) {
            ensure!(props.contains(id.as_str()), "missing related prop {id}");
        }
    }
    for selection in c.selections.values() {
        for member in selection.context["prop_components"]
            .as_array()
            .into_iter()
            .flatten()
        {
            ensure!(
                member["part"].as_str().is_some_and(|id| props.contains(id)),
                "missing assembly part identity"
            );
            ensure!(
                member["selection"]
                    .as_str()
                    .is_some_and(|id| c.selections.contains_key(id)),
                "missing assembly part selection"
            );
        }
    }
    for control in &c.controls {
        for id in &control.choices {
            ensure!(choices.contains(id.as_str()), "missing control choice {id}");
        }
    }
    for choice in &c.choices {
        ensure!(
            controls.contains(choice.control.as_str()),
            "missing parent control {}",
            choice.control
        );
        if let Some(id) = &choice.selection {
            ensure!(
                c.selections.contains_key(id),
                "missing choice selection {id}"
            );
        }
    }
    for selection in c.selections.values() {
        if selection.preview_state == "ready" {
            ensure!(!selection.media.is_empty(), "ready preview has no media");
        }
        for media in &selection.media {
            ensure!(
                !Path::new(&media.path).is_absolute()
                    && Path::new(&media.path)
                        .components()
                        .all(|v| matches!(v, Component::Normal(_))),
                "unsafe media path"
            );
            let path = root.join(&media.path);
            ensure!(
                path.canonicalize()?.starts_with(&canonical_root),
                "media escapes package root"
            );
            ensure!(
                fs::metadata(path)?.len() == media.bytes,
                "media length mismatch: {}",
                media.path
            );
        }
    }
    Ok(())
}

/// Refresh only the browser bundle and manifest lengths, retaining all data and media.
///
/// # Errors
/// Returns an error for an invalid package manifest or failed atomic writes.
pub fn refresh_ui(root: &Path) -> Result<()> {
    let path = root.join("manifest.json");
    let mut manifest: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
    ensure!(
        manifest["format"] == "ghostline-items",
        "not an items catalogue"
    );
    let files = manifest["files"]
        .as_object_mut()
        .context("manifest has no files")?;
    for (name, bytes) in [
        (
            "index.html",
            include_bytes!("../web/dist/index.html").as_slice(),
        ),
        (
            "assets/app.js",
            include_bytes!("../web/dist/assets/app.js").as_slice(),
        ),
        (
            "assets/style.css",
            include_bytes!("../web/dist/assets/style.css").as_slice(),
        ),
    ] {
        atomic(&root.join(name), bytes)?;
        files.insert(name.into(), serde_json::json!(bytes.len()));
    }
    atomic(&path, &serde_json::to_vec_pretty(&manifest)?)
}

/// Atomically replaces one generated file after flushing its contents.
pub fn atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    // Static shards and browser assets are unchanged during most render
    // checkpoints. Preserve them instead of replacing and flushing every file.
    if fs::metadata(path).is_ok_and(|file| file.len() == bytes.len() as u64)
        && fs::read(path).is_ok_and(|existing| existing == bytes)
    {
        return Ok(());
    }
    let parent = path.parent().context("file has no parent")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("create publication directory {}", parent.display()))?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("create publication temporary file for {}", path.display()))?;
    tmp.write_all(bytes)
        .with_context(|| format!("write publication temporary file for {}", path.display()))?;
    tmp.as_file()
        .sync_all()
        .with_context(|| format!("flush publication temporary file for {}", path.display()))?;
    for attempt in 0..8 {
        match tmp.persist(path) {
            Ok(_) => return Ok(()),
            Err(error)
                if cfg!(windows)
                    && attempt < 7
                    && matches!(error.error.raw_os_error(), Some(5 | 32 | 33)) =>
            {
                tmp = error.file;
                std::thread::sleep(std::time::Duration::from_millis(20 << attempt));
            }
            Err(error) => {
                return Err(error.error).with_context(|| format!("publish {}", path.display()));
            }
        }
    }
    unreachable!("publication loop returns or retries")
}
