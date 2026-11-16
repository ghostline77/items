# Asset flow and publication

## Ownership

| Project | Responsibility |
| --- | --- |
| `items` | Catalogue discovery policy, item/customization classification, selection contracts, preview recipes, search/browser UI, manifests and publication packages |
| `ghostline-red` | Archive/cache access, native formats, identifiers, inheritance/property decoding and resource relationships |
| `renderer` | Reusable scene preparation, geometry/materials, rig/morph handling, camera/view control, retained scenes and image generation |
| `character-builder` | Character authoring, combined loadouts/sliders, live assembly and preview interaction; consumes catalogue identities and media |

Game files feed `ghostline-red`. Its decoded resources and resolved bindings feed catalogue discovery and renderer scene preparation. `items` turns selected render outputs and metadata into a published dataset. The browser reads that dataset; it does not extract from the game. Neither native library depends on the catalogue UI.

## Build a catalogue

1. **Select sources.** Identify game build, base/expansion layers, optional mods, localization and explicit precedence. Fingerprint inputs before resolving identities. Keep modded and vanilla provenance distinguishable.
2. **Discover.** Enumerate native item records by class inheritance, character customization roots and referenced appearance/entity/material/morph resources. Follow the coverage checklist, including reusable visual parts with no item record. Retain unexplained entries in the discovery report.
3. **Resolve.** Read native record properties, inheritance and localization through `ghostline-red`. Follow roots, appearances, components, attachments and rig/morph/material dependencies. Preserve customization activation and context relationships. A missing or unsupported link produces a diagnostic rather than an invented default.
4. **Prepare selections.** Deduplicate shared resources; define exact authored variants and explicit preview contexts. Choose useful reference assemblies, color swatches and mounted views. Record the requested coverage instead of expanding every possible loadout combination.
5. **Render.** Feed resolved assets to `renderer`. Run bounded jobs with cancellation, resumable jobs and structured errors. Reuse prepared geometry/textures where possible. Keep labels, source axes and camera recipes explicit.
6. **Verify.** Check referential integrity, native identity precision, view orientation, coverage accounting and missing/failed outputs. Compare representative hair, skin, eyes, clothing/refits and cyberware states against controlled game captures. Record approximation limits instead of treating a completed render as visual equivalence.
7. **Publish.** Build the same versioned metadata and media manifest into hosted and offline packages. Check browser links/search and machine-readable details in both forms. Publish only the selected release artifacts.

Record-table enumeration alone is insufficient for ingestion: native TweakDB flat decoding and property resolution are required for classification and visual links. Format support belongs in `ghostline-red`. The reference [inventory](../coverage/README.md) distinguishes verified table counts from properties that require that deeper decoding.

Discovery and binding resolution run together in `ghostline-items build`; no separate resolution command is required. Every discovered selection is resolved or blocked before publication. Per-selection failures are retained in `data/binding-report.json` and selection details, alongside aggregate resolution counts in `data/coverage.json`. Texture preparation and GPU rendering happen only in the separate render step.

Clothing discovery expands native component manifests without preparing rigs or
mesh material bindings for every owning NPC. Each garment resource contributes
its authored appearances once. The subsequent binding stage resolves those exact
selections before publication. Discovery reports candidate progress, garment
counts, elapsed time and the current resource to standard error; redirect the
native process streams directly when running unattended so progress stays live.

Explicit moved-resource corrections are shared by build-time binding resolution
and rendering and published in selection context. They apply only when the
requested resource is absent and the replacement exists; native requested identities
are retained. Linked skin-color choices for empty genital overlays retain both the
requested palette and the effective empty definition. Empty overlays and root-only
FPP entities have no independent preview geometry and are reported as not applicable.
Missing visible geometry or genuinely absent authored appearances remain explicit
binding failures.

Render recipes may select an explicit native LOD mask for static previews. Geometry
and material preparation use the same selected chunks; lower LODs can have their
own material bindings. The requested masks are published with the preview context
and recorded in renderer inspection notes. This supports authored static geometry
when the highest-detail mesh requires an animation pose outside the preview recipe.

Developer tooling ships as a Rust binary linked to the native libraries. Game installation and source access are generation inputs. Prebuilt catalogue browsing requires neither. Render settings are recorded with each preview.

## Package layout

Each release has a consistent root:

```text
manifest.json
index.html
assets/                  browser code and styling
data/                    JSON entities, indexes and coverage reports
data/browser/            browser-loadable data scripts/chunks
data/<collection>/       bounded JSON arrays indexed by data/catalogue.json
data/browser/details/    on-demand native detail scripts for file:// and HTTP
media/
  images/<resource-name--native-id>/<appearance>/<size>px/<view>.png
```

Native resource names, native IDs and appearance names identify folders. The manifest maps identities to paths, so consumers never parse slugs to recover identity. Preserve icon/swatch media roles separately from rendered views.

The downloadable offline edition includes all referenced media and browser data locally. Opening `index.html` with `file://` must support search and detail navigation without a local server. Browser-loadable data scripts/chunks avoid relying on local-file `fetch`; equivalent JSON remains available to agents and other tools. Hash-based routes support static hosting and direct offline use.

The hosted edition uses the same IDs, relative path layout and UI. A deployment setting resolves media paths either locally or against an R2/custom-domain media base. Deployments must support a GitHub Pages repository subpath. Media host configuration does not become part of asset identity. Offline packaging resolves every referenced path into the package and has no CDN dependency.

## Update and reuse

`render-batch --workers N` prepares and renders N selections concurrently using
independent GPU renderers. There is no fixed worker ceiling; tune the count for
the available CPU, GPU memory and measured throughput. Workers keep separate
native cache receipts under `work/previews/workers/`. A single coordinator copies
finished images into catalogue media and checkpoints catalogue metadata, avoiding
concurrent publication writes. The cancellation file stops all workers cooperatively.
Images and resume receipts commit after each job. Bulk catalogue snapshots share
a two-minute publication clock across recipe groups, with a final snapshot on
completion, cancellation or failure. Unchanged generated files are preserved.
  Bulk rendering removes only media references invalidated by changed recipes or
  incomplete previews. It commits those removals before deleting the superseded
  files and retains any file still referenced by another selection. Checkpoints
  and completion do not scan the full media tree for cleanup.
  Render snapshots retain the built catalogue's binding validation. New images
  are checked and atomically committed before their media references are added;
  unchanged published media is retained. Use `verify` for a full-package audit
  without making every small render retry rescan all published images.

Workers share bounded caches for native mesh decoding and prepared appearances;
each GPU renderer also retains uploaded assets for repeated reference geometry.
Native archive payloads and resized bake inputs have bounded memory caches, and
appearances share an extracted material repository. Embedded resources remain
scoped to their owning mesh. PNG outputs use fast lossless compression.

Bulk rendering publishes an initial catalogue only when it invalidates existing
visible media or changes nonvisual classifications. Unchanged pending work starts
without duplicating the entire metadata publication; per-job receipts and timed
checkpoints retain resumability.
Assembly readers share reflected documents, and appearances share immutable mesh
streams, morph targets and material-source documents rather than copying them.
Supersampled images are reduced and converted to straight-alpha sRGB on the GPU,
with only the final image dimensions transferred back for PNG encoding.
Bulk rendering reuses published previews when their recipes and media file sizes
match, including across executable upgrades. Changed recipes are rendered again;
partial renderer work remains protected by its build and input fingerprints.

Recipes orient isolated rigid wearable eyewear using native head-bone axes; loose
world and quest props retain their authored world axes. Recipes frame a single
hand for strong-arm knuckle details, and crop the native marker's underground
helper geometry out of the camera bounds. Material revisions invalidate affected
published previews when coverage masks, native tint interpretation or lighting
change. Regenerate the plan to apply these recipes to existing catalogues.

Paired cuffs, gloves and blade details use one native X half for camera bounds;
the other side stays at its authored placement. Nipple surface variants use the
same close-up mode with a side view. Weapon recipes fit projected bounds and
include lateral views. Native skin tints and strand profiles remain source data;
a smooth or pale native variant is not replaced with invented pigment or shape.
Pubic-hair recipes include a native genital selection from the same customization
root and linked body palette: vaginal context for the female preview frame and
penile context for the male preview frame. This preview policy does not constrain
which native anatomy the character builder may assemble. The context contributes camera bounds;
the larger body remains reference context outside those bounds.

After a game patch, rebuild metadata from the selected native inputs and schedule the required preview jobs. The renderer manages resumable job output. Cancellation leaves job state usable for a resume; incomplete files are never published as ready media.

Rebuild the catalogue in its existing output directory. Native identities remain the record IDs, resource paths, appearance names and source locators read from the selected inputs.

`character-builder` can bundle or download a compatible dataset and use its previews immediately. Live previews resolve the selected native bindings through `ghostline-red` and `renderer`, with the builder supplying its assembled body, slider and loadout context. Catalogue previews do not need to enumerate those complete characters.
