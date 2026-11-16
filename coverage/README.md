# Coverage evidence

These inventories establish source coverage for the catalogue model. They do not contain a generated catalogue or game assets.

| File | Meaning |
| --- | --- |
| [documented-customization.json](documented-customization.json) | 38 documented base control groups and established extension slots, with a pinned documentation source |
| [game-customization.json](game-customization.json) | Native control definitions from four base/Phantom Liberty customization roots, including classes, slots and selected choice counts |
| [game-item-records.json](game-item-records.json) | Native TweakDB record-table counts, file fingerprints, inheritance classification and base/EP1 overlap |
| [item-type-reference.json](item-type-reference.json) | Reflected category/type vocabulary; presence in this vocabulary does not prove a populated record |

The selected installation is Cyberpunk 2077 2.31. Customization resources were decoded with `ghostline-red` from the unmodified `content` and `ep1` archive roots. The decoder and decoding schema are fingerprinted. Decoded JSON hashes identify the local inspection artifacts; semantic root hashes exclude the serialization envelope. Raw dumps remain local build intermediates.

Item counts come from the record sections of `r6/cache/tweakdb.bin` and `tweakdb_ep1.bin`: blob version 8, parser version 4, with 12-byte ID/type entries. Type hashes use Murmur3 x86_32 with seed `0x5EEDBA5E`; reflected class inheritance identifies `gamedataItem_Record` descendants. Every type hash in the inspected record tables resolved. Flat properties, display names, availability and visual bindings are not established by counting record entries.

Sources overlap: all 8,417 base item IDs also occur in the EP1 table. The union is 10,042 IDs. Sources must be layered with explicit precedence when resolving properties; these counts alone do not establish that precedence.

Changing source files requires a new inventory and fingerprints. A claim of complete coverage must name its source scope and account for unresolved records, resources, controls and previews separately.
