# Catalogue coverage

## Source scope

A catalogue release identifies its game version, selected archives, expansion content, optional mods and source filenames. Documentation provides the discovery checklist; native records and resources establish what that release actually contains. Unknown classes, slots and relationships remain visible in coverage reports.

The reference inventory covers Cyberpunk 2077 2.31 base and Phantom Liberty data, excluding mod archives. It contains 8,417 base item records and 10,042 EP1 item records, with a union of **10,042 native item IDs**. This is a record-table count, not a count of selectable looks or rendered previews. [Inventory and method](../coverage/game-item-records.json).

## Furniture and world props

Native world entities, meshes and morph geometry can be catalogued independently
of gameplay item records. Coverage reports count prop resources, complete native
assemblies, individual meshes and exact appearance selections separately. Resource
name hints identify discovery candidates; installed archives and decoded resources
establish existence, native membership and placements.

Furniture, storage, seating, tables, appliances, devices, tools, containers and
decorative clutter use the same discovery and preview pipeline. Complete entries
follow native entity definitions and their appearances/parts; referenced meshes
remain individually searchable, including dependencies outside the requested depot
prefix. Native instance counts and rig/slot transforms are preserved. Merely sharing
a filename prefix does not imply that a body, door or drawer belongs in an assembly.

## Player and NPC clothing

Clothing discovery includes installed base/EP1 garment meshes, garment entities and
their exact appearances, independently of gameplay equipment records. Native
character entity/appearance components also identify garments outside the garment
folders. The Clothing collection retains these resources and their exact selection
links; source character/component references identify where a mesh appearance is
used. Native paths and component evidence do not imply compatibility with V.

Catalogue builds resolve clothing geometry and material bindings before publication.
The clothing render family previews isolated garments with geometry-based framing;
shadow-only geometry remains explicitly nonvisual. Missing resources, appearance
errors and unsupported native assembly behavior remain reported.

## Gameplay records

Retain every selected `gamedataItem_Record` descendant, including items without a model or a player spawn identity. Preserve concrete record class, inheritance and native classification before assigning browser categories.

| Family | Coverage |
| --- | --- |
| Clothing | Head, face, inner/outer torso, legs, feet, outfits; appearance variants, component bundles, body refits and garment fitting |
| Weapons | Ranged, melee, throwable and mounted/vehicle weapon item records; appearances, components and attachment relationships |
| Parts and modifications | Scopes, muzzles, magazines, rails, receivers, stocks, targeting, weapon mods, fabric enhancers and other native attachment slots |
| Gameplay cyberware | Implant records, arm systems, tiers, upgrades, equip areas, slots and linked effects; visual bindings where present |
| Consumables and gadgets | Food, drink, inhalers, injectors, ammunition, grenades, gadgets and their native subtypes |
| General and progression | Crafting materials, recipes, programs, shards, readable/key/quest/general items, vendor tokens and other native classes |

The reflected vocabulary has eight item categories and 102 item types. It is a discovery aid, not a closed schema or proof that all types occur in the installed game. Preserve unfamiliar values. [Vocabulary](../coverage/item-type-reference.json).

Item record names, root entity appearances and mesh material appearances are different identities. A record can share geometry with many other records; a visual asset can have no record. Inventory icons are separate media and do not imply an available 3D model. Native weapon visibility may depend on animation parameters as well as component masks.

References: [item structure][item-structure], [equipment slots][equipment-slots], [spawn identities][spawn], [attachment slots][attachment-slots], [weapon attachments][weapon-attachments], [cyberware records][cyberware].

## Character customization

The catalogue includes reusable player and NPC visual parts, authored options and the controls needed to select them. Availability in a player menu is recorded separately from asset existence.

| Domain | Coverage |
| --- | --- |
| Hair | Hairstyles, beard parts, brows, lashes, hair/beard/brow/lash colors and profiles; cap/hat hair, companion parts and dangle rigs |
| Face geometry | Head/donor assets, eyes, ears, nose, mouth, jaw, teeth and morph regions, including baseline states |
| Skin | Skin tone, complexion/type, body materials and linked head/body/arm color controls |
| Cosmetics | Eye, lip and cheek makeup; style/type/color controls; blemishes and color; scars, face/body tattoos, overlays and body hair |
| Cosmetic cyberware and jewelry | Facial chrome, piercings, piercing colors and their component/morph/material bindings |
| Body and anatomy | Body frames, breasts, nipples, genitals and size morphs, pubic hair and colors; modesty/censor controls |
| Arms, hands and feet | Ordinary and cyberware arms, left/right nails and colors, holstered/deployed states, flat/high-heel foot states and seam companions |
| Context and extensions | FPP, TPP, creator and photo-mode groups; refit/body tags; declared mod-provided slots, styles and material extensions |

Voice, lifepath, attributes, perks and complete authored character presets belong to character-building tools. Relevant native bindings can be retained as metadata; the catalogue owns visual discovery and exact selection data.

The documentation lists 38 base control groups covering 41 slot strings. Its cheat sheet explicitly warns that older option lists omit patch 2.2 additions. Consequently neither its numeric menu counts nor its slot names define catalogue completeness. The full documented checklist, including ten established additional tattoo slots, is retained in [documented-customization.json](../coverage/documented-customization.json).

References: [customization resource structure][cc-resource], [CCXL controls][ccxl], [switcher behavior][switchers], [head parts][head], [body parts][body], [arms][arms], [morphs][morphs], [body refits][refits], [character cheat sheet][cc-cheat].

## Verified differences and additional controls

The four native roots in [game-customization.json](../coverage/game-customization.json) supply the following evidence:

| Root | Control definitions | Distinct nonempty UI slots |
| --- | ---: | ---: |
| Base female | 510 | 60 |
| EP1 female | 510 | 60 |
| Base male | 558 | 55 |
| EP1 male | 558 | 55 |

These are definitions across head/body/arm contexts. They are not visible menu rows, unique assets or a denominator for render completion. Equal counts do not establish equal contents.

- Native hair uses `hairstyle`; the CCXL table calls it `hairs`. Both `hairstyle` and the disabled `hairstyle_cyberware` control have 51 switcher options in the inspected roots.
- `eyelash_color` is a native appearance control with 35 definitions. Eyelash style extensions are a separate concern. The inspected `eyes_color` control has 71 definitions and `nails_color_tpp` has 54.
- `genitals_size` includes appearance and morph controls. A slot therefore cannot uniquely identify a control or determine its kind.
- `skin_color` has 12 definitions but no explicit resource on the control. Targets can depend on surrounding assembly/context bindings.
- A morph entry can omit its morph name to select the baseline shape. This is distinct from turning an asset off or failing to resolve a binding.
- Switcher choices activate multiple named controls. Nested skin/lip controls, groups, perspective mappings and link controllers must survive ingestion.
- Internal controls include foot states, holstered arm data, underpants, neck and scene bruises. Keep them in the source graph and classify their role instead of exposing every definition as a user-facing menu.

The inventories verify definitions and counts. Resource traversal, contextual composition and game screenshots establish whether a resolved selection reproduces the intended look. Where static resources leave behavior ambiguous, compare a controlled in-game selection, retaining the source selection, perspective, screenshot and conclusion as evidence. Do not infer runtime behavior from a successful decode alone.

## Completeness and previews

Report records, visual assets, control definitions, authored choices, resolved selections and preview jobs separately. Every discovered source entry must be classified or listed as unresolved. Every eligible preview must be ready, pending, failed or blocked with a reason; metadata-only entries can be explicitly not applicable.

Cover authored styles, colors, variants and relevant reference contexts without enumerating every possible character loadout. Swatches and mounted reference examples can represent material choices; they must still link to exact native bindings. Hair, arm and clothing previews name their body/rig context and required companion parts. Compatibility remains unknown unless supported by native bindings, declared refit tags or verification evidence.

[item-structure]: https://github.com/CDPR-Modding-Documentation/Cyberpunk-Modding-Docs/blob/a54b08735a560bca7dd76b1e1dab1631e6aaae89/modding-guides/items-equipment/adding-new-items/archive-xl-item-structure-explained.md
[equipment-slots]: https://github.com/CDPR-Modding-Documentation/Cyberpunk-Modding-Docs/blob/a54b08735a560bca7dd76b1e1dab1631e6aaae89/modding-guides/items-equipment/adding-new-items/different-equipment-slots.md
[spawn]: https://github.com/CDPR-Modding-Documentation/Cyberpunk-Modding-Docs/blob/a54b08735a560bca7dd76b1e1dab1631e6aaae89/for-mod-creators-theory/references-lists-and-overviews/equipment/spawn-codes-baseids-hashes.md
[attachment-slots]: https://github.com/CDPR-Modding-Documentation/Cyberpunk-Modding-Docs/blob/a54b08735a560bca7dd76b1e1dab1631e6aaae89/for-mod-creators-theory/references-lists-and-overviews/cheat-sheet-tweak-ids/cheat-sheet-attachment-slots.md
[weapon-attachments]: https://github.com/CDPR-Modding-Documentation/Cyberpunk-Modding-Docs/blob/a54b08735a560bca7dd76b1e1dab1631e6aaae89/modding-guides/items-equipment/adding-new-items/weapons/weapons-attachments-and-scopes.md
[cyberware]: https://github.com/CDPR-Modding-Documentation/Cyberpunk-Modding-Docs/blob/a54b08735a560bca7dd76b1e1dab1631e6aaae89/for-mod-creators-theory/tweaks/creating-cyberware-using-tweakxl.md
[cc-resource]: https://github.com/CDPR-Modding-Documentation/Cyberpunk-Modding-Docs/blob/a54b08735a560bca7dd76b1e1dab1631e6aaae89/for-mod-creators-theory/files-and-what-they-do/file-formats/character-creator/README.md
[ccxl]: https://github.com/CDPR-Modding-Documentation/Cyberpunk-Modding-Docs/blob/a54b08735a560bca7dd76b1e1dab1631e6aaae89/for-mod-creators-theory/core-mods-explained/archivexl/archivexl-character-creator-additions/README.md
[switchers]: https://github.com/CDPR-Modding-Documentation/Cyberpunk-Modding-Docs/blob/a54b08735a560bca7dd76b1e1dab1631e6aaae89/for-mod-creators-theory/core-mods-explained/archivexl/archivexl-character-creator-additions/ccxl-theory-switchers.md
[head]: https://github.com/CDPR-Modding-Documentation/Cyberpunk-Modding-Docs/blob/a54b08735a560bca7dd76b1e1dab1631e6aaae89/for-mod-creators-theory/references-lists-and-overviews/cheat-sheet-head/README.md
[body]: https://github.com/CDPR-Modding-Documentation/Cyberpunk-Modding-Docs/blob/a54b08735a560bca7dd76b1e1dab1631e6aaae89/for-mod-creators-theory/references-lists-and-overviews/cheat-sheet-body.md
[arms]: https://github.com/CDPR-Modding-Documentation/Cyberpunk-Modding-Docs/blob/a54b08735a560bca7dd76b1e1dab1631e6aaae89/for-mod-creators-theory/references-lists-and-overviews/cheat-sheet-arms.md
[morphs]: https://github.com/CDPR-Modding-Documentation/Cyberpunk-Modding-Docs/blob/a54b08735a560bca7dd76b1e1dab1631e6aaae89/for-mod-creators-theory/3d-modelling/morphtargets.md
[refits]: https://github.com/CDPR-Modding-Documentation/Cyberpunk-Modding-Docs/blob/a54b08735a560bca7dd76b1e1dab1631e6aaae89/for-mod-creators-theory/core-mods-explained/archivexl/archivexl-body-mods-and-refits/README.md
[cc-cheat]: https://github.com/CDPR-Modding-Documentation/Cyberpunk-Modding-Docs/blob/a54b08735a560bca7dd76b1e1dab1631e6aaae89/for-mod-creators-theory/references-lists-and-overviews/cheat-sheet-character-creator.md
