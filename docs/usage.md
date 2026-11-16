# Build and browse

## Developer inputs

Build the CLI with `cargo build --release --locked`. The executable is `target/release/ghostline-items.exe` on Windows. The sibling `ghostline-red` and `renderer` crates are linked into it.

Generation requires an installed game. RED reflection metadata is bundled and loaded automatically; `--schema` is an optional advanced override. Equipment and character customization additionally require one explicitly selected complete compiled cache. Select `r6/cache/tweakdb.bin` for base-game records or `tweakdb_ep1.bin` for the expansion cache; do not sum or blindly overlay them. The archive reader selects installed unmodified base/EP1 archives. Runtime mod substitutions are outside this source scope.

An optional `--tweak-names` input supplies native name hints in the length-prefixed dictionary format, optionally KARK compressed. Names are assigned only when their hashes match real record/flat identities. Without it, record IDs and direct property lookups remain usable, while names and properties discoverable only through the dictionary remain unresolved. These inputs are developer data, not dependencies of catalogue browsing.

```powershell
./target/release/ghostline-items.exe build --scope all --game "C:/Games/Cyberpunk 2077" --tweakdb "C:/Games/Cyberpunk 2077/r6/cache/tweakdb_ep1.bin" --tweak-names tweakdbstr.kark --resource-names depot-paths.kark --game-version YOUR_GAME_VERSION --out dist/catalogue
```

Replace `YOUR_GAME_VERSION` in examples with the installed game's version label.
`--game-version` is the declared build label. The catalogue records the selected game version, native cache filename and optional name dictionary filename. Output must be outside the game installation and either empty or an existing generated items package. The builder never edits installed game files.

## Search and native identities

`build --scope all` includes equipment, customization, player/NPC clothing and props. Use `--scope items`
for equipment/customization/clothing, or `--scope props` for an independent prop package
without TweakDB. Prop discovery requires `--resource-names`: UTF-8 newline-separated
native depot path hints, optionally KARK compressed. The installed archive index
establishes which resources actually exist. No external extraction application is
invoked or needed for generation.

Default prop roots cover environments, items, devices, meshes, props and prefabs
in base/EP1. Repeat `--prop-prefix` to choose a narrower or different inventory.
Referenced assembly parts are included even when they lie outside those prefixes.

Normal `all` and `items` builds also discover player and NPC clothing from native
resources, including garments without equipment records. `--resource-names` is
required for this inventory. `--scope clothing` builds a standalone clothing
catalogue without TweakDB; `plan-renders --family clothing` selects its previews.
The browser Clothing collection and CLI search expose these resource identities
and all authored appearances. Clothing previews preserve the native garment shape;
they do not assert player-body compatibility.

`build --scope all --resume` resumes an existing discovery snapshot in `--out`,
retrying blocked bindings and unfinished assembly placements while retaining
completed selections and previews. It checks the archive/schema identity and
declared version/language. Use the same TweakDB and name inputs as the original
build; use a fresh build when those inputs or the discovery scope change. Binding
fixes are part of normal generation and apply to fresh catalogues as well.
For example:

```powershell
./target/release/ghostline-items.exe build --scope props --game "C:/Games/Cyberpunk 2077" --resource-names depot-paths.kark --prop-prefix base/environment/decoration/furniture/ --prop-prefix base/items/interactive/furniture/ --game-version YOUR_GAME_VERSION --out dist/furniture
./target/release/ghostline-items.exe plan-renders --catalogue dist/furniture --family props --out work/furniture.json
./target/release/ghostline-items.exe render-batch --catalogue dist/furniture --game "C:/Games/Cyberpunk 2077" --plan work/furniture.json --work work/furniture-renders --workers 16
```

The Furniture & props collection distinguishes **Complete assemblies** and
**Individual meshes**. Native entity assemblies include their component instances,
rig/slot placements, enabled states, chunk masks, appearance overrides and material
bindings. A cabinet entry includes all native drawer instances; selecting its drawer
mesh opens that part independently. Containing-assembly and part links work in both
directions. Filename similarity never establishes membership or placement.

Assembly previews use the native rig rest pose, not the character-isolation recipe.
Individual meshes render directly. Build-time placement validation needs neither
material texture baking nor a GPU. World-sector-only arrangements without a native
entity definition are not inferred from nearby resources.

Open the generated `index.html` to browse equipment, native customization controls, authored choices and visual resources. Search supports human labels, native item names/IDs, appearance names and resolved mesh identities. Category and native-slot filters keep equipment and customization discoverable. Full details load from local script shards, without an HTTP API or JSON fetch requirement.

CLI search emits JSON lines; details emit one JSON object:

```powershell
./target/release/ghostline-items.exe search --catalogue dist/catalogue "Vest_17_basic_02"
./target/release/ghostline-items.exe show --catalogue dist/catalogue Items.Vest_17_basic_02
```

Search results list native resource paths, exact appearances and render arguments. Inspect the native variant before rendering: its root, named appearance and component bindings identify the actual variant. Menu position, human name and thumbnail filename are not selection identities.

## Build bindings and render

`build` automatically resolves every discovered selection's components, meshes, masks, rigs, attachments and exact mesh-appearance material slots. It reuses native documents across appearances of the same root and compact mesh-binding results across roots, without baking textures or allocating a GPU. Each selection finishes as resolved or blocked; one failed selection does not stop the rest of the build.

`data/coverage.json` reports resolution totals. `data/binding-report.json` lists blocked selection identities and their errors, item discovery diagnostics and source diagnostics. Partial component data remains available on blocked selections. Root entity/appearance selections are exact. Component mesh appearances follow the native rule: an exact name wins; otherwise the first authored mesh appearance is used. Material bindings record the request, effective appearance and whether that native default was used. A completed build may contain blocked entries; inspect the report before choosing a render set.

Rendering is the separate generation step. Select `--item` and an exact `--appearance`, or `--resource` and `--appearance` copied from search results. `--all` renders the resolved selections using the requested recipe and skips blocked selections. Rendering requires a completed catalogue build. `render` uses native names and writes hero/front/back PNGs:

```powershell
./target/release/ghostline-items.exe render --catalogue dist/catalogue --game "C:/Games/Cyberpunk 2077" --item Items.Vest_17_basic_02 --appearance "t2_vest_17_basic_02_&Female&TPP" --work work/previews --size 512
```

Use `--reference-resource` and `--reference-appearance` with another native resource/appearance pair to display native reference geometry, such as a head behind hair. References are recorded separately from the selected asset. The renderer uses an explicit bind-pose recipe: authored geometry and local transforms, without evaluating missing external character rigs or bone attachments. A reference head does not establish universal body fit.

For hair previews, use `--neutral-head female` or `--neutral-head male`. This adds the corresponding unmorphed native player base head with a texture-free matte grey material. Choose the frame explicitly; it is not inferred from a hairstyle filename. This option replaces the custom reference options. The native head resource and baseline morph state are recorded in the preview context, and media paths distinguish the two frames. The head remains separate from the exported hair selection.

```powershell
./target/release/ghostline-items.exe render --catalogue dist/catalogue --game "C:/Games/Cyberpunk 2077" --resource "base/characters/head/player_base_heads/appearances/hairs/hh_004_pwa__hairs_090.app" --appearance 05_brown_liquorice --neutral-head female --work work/previews --size 1024
```

Each newly prepared assembly job writes `assembly-inspection.json` in its work directory. It records native component and rig bindings, effective transforms and chunk visibility, native and GPU triangle counts, material inputs, head geometry bounds and effective camera matrices. Texture paths describe preparation intermediates, which are temporary. The job's renderer build identity ties those inputs to the studio shader and image conversion. Inspect the actual hero/front/back images as well as this report: successful rendering alone does not verify appearance fidelity.

Re-run the same request with the same work directory to resume. The shared renderer manages resume receipts for its own jobs. A changed recipe uses a different cache entry. `--cancel-file path` stops cooperatively when that file exists. Jobs run serially with scene/resource bounds; large manifests are split into bounded groups. Native receipts and per-selection checkpoints survive interruption. The published dataset is refreshed when the batch ends, including successful outputs and failure diagnostics; render failures produce a nonzero exit status.

`build` refreshes metadata, resolves bindings and refreshes coverage; rendering is a separate operation. An existing catalogue retains ready previews when the selected source declarations, resolved native components and material bindings are unchanged. Changed bindings invalidate their media, and publication removes unreferenced images. Keep the render work directory to reuse compatible outputs after a metadata rebuild. Rebuild metadata when changing the game sources or schema.

For the same game source snapshot, `render-batch --resume-from work/previews`
can republish completed cached frames after the published media has been removed.
Repeat `--resume-from` for additional completed repair batches, in chronological
order. Native bind-pose inputs and saved frame sizes are checked before restoring;
the current plan then validates recipes and renders outstanding selections.

## Bulk render plans

Generate a reviewable manifest from the built catalogue. Planning needs only the published data, not a game installation:

```powershell
./target/release/ghostline-items.exe plan-renders --catalogue dist/catalogue --out work/renders/all.json --family all --size 1024
./target/release/ghostline-items.exe plan-renders --catalogue dist/catalogue --out work/renders/hair.json --family hair --size 1024
```

Every resolved selection in scope receives a concrete recipe. Equipment and independent objects use isolated native geometry. Hair, including FPP geometry, uses a neutral head selected from its owning native customization root. Facial parts use neutral-head context with hero/front close-ups. Body and arm parts use the frame's native TPP body-color baseline with neutral materials and subject framing. Geometry and chunks already supplied by the subject are removed from the neutral reference to avoid overlapping surfaces. These are preview contexts; they do not assert universal compatibility.

Plans retain selection IDs, owning control IDs, source declarations, output size, semantic views and exact reference selections. Recipes can be edited to select a different native reference. Actual binding failures remain listed as outstanding work; missing recipe code is not an exclusion category.

Run a bounded range before a larger batch, or omit the range options to render the whole manifest:

```powershell
./target/release/ghostline-items.exe render-batch --catalogue dist/catalogue --game "C:/Games/Cyberpunk 2077" --plan work/renders/hair.json --work work/previews --offset 0 --limit 10 --cancel-file work/stop-rendering
```

The runner groups identical recipes and uses the shared renderer's bounded jobs and validated resume receipts. Repeating the same command reuses valid outputs and retries failures; outcomes distinguish newly written and reused frames. Remove the cancellation file before resuming. Native render failures do not prevent later jobs or recipe groups from running; the command reports failure after preserving successful output. Fatal source/publication errors stop the run.

`work/previews/render-progress.json` updates after every job. `work/previews/bulk-report.json` records completed recipe groups, outcomes and cancellation. Per-job native receipts and `catalogue-selection.json` checkpoints remain in the work directory. The static catalogue is refreshed after each bounded renderer chunk. Publication removes unreferenced files from its generated `media` directory, so failed or replaced renders are not retained there. The validated resume cache remains separate in `work`.

Media folder names use reversible `~HH` byte escaping for punctuation and uppercase characters, avoiding native-name collisions and Windows filename restrictions. Consumers use the published media paths and native IDs directly.

## Verify and package

```powershell
./target/release/ghostline-items.exe verify --catalogue dist/catalogue
./target/release/ghostline-items.exe package --catalogue dist/catalogue --out dist/offline
./target/release/ghostline-items.exe package --catalogue dist/catalogue --out dist/hosted --media-base https://media.example.com/items
```

Verification checks references, data/browser files, native references and media lengths. Both package forms include their local media; `--media-base` changes the hosted browser's URL resolution. The default empty media base keeps the offline package self-contained. Deploy the resulting root under a domain or GitHub Pages subpath; no backend process is required.

`data/catalogue.json` is a descriptor with source metadata, coverage and collection shard paths. Each collection shard is a JSON array; assets and selections have IDs for indexing. Browser summaries are separate from full details. `manifest.json` lists the active data/browser files and their lengths. Media entries carry their byte lengths, native selection, context recipe and semantic view.

## Interpreting coverage

Records, assets, controls, authored choices, exact selection candidates, resolved assemblies and previews are counted separately. Base and EP1 customization roots retain separate ownership even when their contents overlap. Native records without visual roots remain searchable. A resolved assembly can still require external context or fail material preparation; a completed render is an approximation, not proof of game-equivalent appearance.

The catalogue retains source limitations and failed bindings. Empty/context-only definitions, missing authored appearances and unsupported transforms remain explicit diagnostics rather than being replaced with unrelated variants.
