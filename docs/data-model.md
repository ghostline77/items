# Catalogue data model

## Identity and provenance

The catalogue has a schema version and records its game/source version. Its manifest names the game build, source layers and their precedence, selected archive/cache files, localization languages and generation inputs. A record's origin identifies its owning source layer; resolved values retain their source and inheritance chain.

Use native identities directly. An item retains its TweakDB record identity and known record name. A visual selection is keyed by its resource path and exact appearance name. A customization control or choice uses its owning resource and native source locator. No additional identity hash is generated. Source locators are scoped to the source snapshot when a patch changes array layout. Human labels and shortened filenames remain presentation fields.

Native identifiers retain their namespace: TweakDB IDs, resource path hashes, localization keys and catalogue IDs are not interchangeable. Export 64-bit integers and masks as strings to preserve exact values in browsers. Keep a known native name alongside its hash; unknown names do not invalidate a known hash. Resource paths retain native spelling, with canonical comparison rules handled by the native resource layer.

Every entity can carry evidence references, diagnostics and resolution state. Unknown, absent, disabled, baseline and not applicable have distinct meanings. Missing serialized fields remain distinguishable from explicitly stored defaults until the native class defaults are known.

## Entities

| Entity | Required identity and content | Relationships |
| --- | --- | --- |
| Item record | Native record ID, known name, concrete class, source; inherited/resolved properties | Visual assets/selections, localization, icons, upgrades, attachments and related records |
| Visual asset | Native resource identity and role: clothing, hair, head, arm, cosmetic, weapon part or another discovered role | Root entity/appearance, geometry, morph/material/profile resources, item records and customization choices |
| Customization control | Owner resource, head/body/arm domain, native control name and class, source locator | UI slot(s), groups, nested choices, links, activation/deactivation and target bindings |
| Customization choice | Owning control and native option locator; switcher option, appearance definition or morph entry | Activated controls, appearance/morph/material targets, labels, icons, tags and randomization metadata |
| Selection | Exact resolved visual configuration and its source identity | One or more assets/choices, component bindings, context and preview records |
| Preview context | Body/rig/resource identities, perspective and component state | Reference head/body/arms, companion parts, declared fit tags and compatibility evidence |
| Preview | Selection, context, view and render recipe | Output media, state, diagnostics and rendering limitations |

These are logical entities. Storage can normalize shared data and publish browser indexes without changing their identities or duplicating every shared resource into every result.

### Item record

Preserve native category, type, equip area, placement/attachment slots, tags, localization keys, icon references and relevant stat/effect/upgrade relationships. Retain unsupported properties in source-backed technical metadata or an explicit unresolved report. Browser families are an additional classification, not a replacement for native values.

Separate a resolvable TweakDB identity from verified spawnability, player availability and appearance availability. Recipe, program, money and abstract/template records can be useful metadata-only entries. A gameplay implant and cosmetic facial chrome may have related labels but are separate entities unless source evidence links them.

### Customization control and choice

Identify a control by its owning resource and native location/name, not just `uiSlot`. Retain its class and the native option collection kind (`options`, `definitions` or `morphNames`). Preserve group membership, `uiSlots`, `link`, `linkController`, `onDeactivate`, edit tags, defaults, hidden/enabled flags, censor rules and perspective mappings where present.

Choice indices are native selection parameters within a source snapshot. Preserve explicit indices and omitted-index semantics, localized labels, icon/swatch IDs, tags and activation targets. A switcher option can activate several controls, and an appearance or morph choice can have context-dependent targets. A missing direct resource path is not proof of a missing asset.

For example, the inspected `hairstyle` switcher has an option activating both `hair_color1` and `hair_color_fpp_01`. A consumer needs that relationship, not just a label such as “01.” The inspected `genitals_size` slot contains both appearance and morph controls. A baseline morph with no name is a valid selection, distinct from an off choice.

Keep internal controls in the source graph and distinguish them from selectable browser controls. Declared mod extensions retain their dependency/source scope rather than becoming assumed vanilla options.

### Selection and context

A selection identifies the root resource, exact entity/appearance name and applicable component bundle. Component bindings retain mesh/morph resources, mesh appearances, chunk masks, local transforms, rigs/attachments, morph name/region/weight and material/profile overrides as applicable. Keep source bindings separate from renderer-prepared GPU data.

Context names the body frame, rig, refit tags, FPP/TPP/creator/photo-mode perspective, arm deployment state and any required head/body/arm or seam companions. Selected assets and reference companions have separate roles so a preview mannequin does not become part of an exported clothing selection.

Different records may point to the same selection; one record may have several selections. A color profile may affect multiple material roles. Avoid fabricating an independent item record for each such combination.

Compatibility states are supported, incompatible or unknown, with evidence. A filename's apparent body/gender suffix is discovery metadata, not sufficient evidence of fit. Runtime-driven states that cannot be resolved statically remain explicit constraints.

## World props and complete assemblies

`props` is a separate collection from gameplay item records and character controls.
A prop ID is `prop:<native resource hash>`; its exact selections retain the existing
root-resource/appearance identity. `kind` distinguishes native entity `assembly`
entries from individual `mesh` entries. Category and readable names are presentation
metadata; the original resource spelling and hash remain available.

`parts` links an assembly to unique mesh prop identities. `assemblies` provides the
reverse relation. Repeated instances are retained separately in each selection's
`context.prop_components`: component name, part ID, exact mesh selection, and
`native_world_transform`. These matrices are column-major, entity-relative, in
metres and native RED Z-up coordinates, evaluated from native local transforms,
parent bindings, rig rest poses and slot offsets. Four uses of one drawer mesh have
four instance records and four native placements. `components` retains the original
hierarchy and rig bindings rather than replacing them with flattened guesses.

`context.prop_kind` selects the render path. Entity assemblies evaluate native
attachments and rigs; standalone meshes retain their own geometry and material
appearance. Failed placements block the assembly during catalogue generation and
are included in the same binding report as other asset failures.

Standalone parts can declare `standalone_origin_anchors` for attachment frames
provided by their runtime owner. These frames are frozen at the preview origin;
native mesh-local placements stay intact. `placement_notes` records this context
and any authored bind-geometry preview used when a runtime skinning rig is absent.
Such previews do not claim to reconstruct an owning object's animation state.
Empty native meshes and assemblies without visible geometry remain resolved,
nonvisual entries. Physics-decoration resources retain their authored mesh data
and material appearances alongside their physics metadata.

An explicit `binding_contexts` list can supply an owning native entity's transform
and rig dependencies to a standalone part without adding the owner's mesh geometry.
Audited `slot_bone_overrides` retain the requested source binding and record the
native frame used for the preview. Physics bone attachments use their authored
mesh bind matrices when no animation rig supplies the named frame. Geometry without
material assignments can use a labelled neutral diagnostic preview; this does not
invent an authored surface or turn an empty/shadow-only resource into visible media.

## Consumer contract

Search results expose native identity, entity kind, localized label, native identity, classification and preview status. Details expose the complete linked native selection and technical metadata. Search indexes include record names/hashes, appearance names and resource/mesh paths as well as human labels.

A character-builder selection names the native resource, appearance and source context, plus the intended context. The dataset carries enough native bindings to resolve it against the corresponding installed source. Consumers validate source/dependency compatibility before assembly; they do not reconstruct a selection from a display name or thumbnail filename.

Previews are optional for consumption. An unresolved asset can remain discoverable with a reason, while a fully resolved selection can be consumed before its image is produced. Agent-readable JSON and human detail pages describe the same identities.

Catalogue generation resolves every discovered selection before publication. Selection resolution is either resolved or blocked, with per-component diagnostics where available. `context.material_bindings` records native mesh/morph geometry, its material-source mesh, requested and effective appearances, native first-appearance fallback and chunk material names. Component details retain whether the authored mesh appearance was omitted. Blocked selections retain any successfully discovered components and bindings; their failures remain outstanding in render plans. `data/binding-report.json` collects these failures without making image generation a prerequisite for usable catalogue data.

## Media and coverage

Media entries use relative paths, MIME type, dimensions, content hash and a semantic view name. `hero` is a presentation choice; `front` and `back` describe the subject's orientation. A render recipe specifies camera coordinates/orientation and source-to-render transforms so labels can be verified independently of mesh rotation conventions.

Preview state is one of pending, ready, failed, blocked or not applicable. Ready outputs name their native selection, reference context, image size and semantic views.

Coverage measures discovery, classification, resolution and rendering separately. Counts include source entries, deduplicated records/assets, controls/choices, resolved selections, eligible preview jobs and state totals. Reports identify unresolved links, missing resources, unsupported materials and evidence limitations. An image count alone does not establish complete coverage.
