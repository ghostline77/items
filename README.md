# Ghostline Items

A searchable Cyberpunk 2077 equipment, character customization, furniture and prop catalogue for people, agents and building tools.

The catalogue covers gameplay records and visual customization assets: clothing, weapons, attachments, cyberware, hair, heads, body parts, cosmetics and their authored variations. Record IDs, native selection identities, resource paths and previews travel together so a result can be used beyond the catalogue browser.

The Rust CLI builds catalogue packages directly from native game data, resolves exact visual bindings and runs resumable preview jobs through the shared renderer. The static browser searches the generated data and loads full native details on demand.

World props have separate entries for complete native entity assemblies and individual
mesh resources. Assemblies retain every component instance and its resolved native
placement, including repeated drawers, doors, labels and contents. Parts link back
to their containing assemblies; membership and offsets come from native resources.

## Browse a published catalogue

Open the package's `index.html` to search items and native appearances. Keep its
data and media directories together. The offline edition needs neither the game
nor the developer tools.

## Generate a catalogue

Use Rust 1.92+, an installed game and the sibling native crates declared in
[Cargo.toml](Cargo.toml). Install the parent JavaScript workspace with `npm ci`
before building a fresh checkout, then run `cargo build --release --locked`
from this repository.

[Build and browse](docs/usage.md) covers required name dictionaries, source-cache
selection and commands. Catalogue generation resolves visual bindings and reports
failures automatically. Render previews separately, review the output, then verify
and package the catalogue.

## Documentation

- [Preview families, framing and reference geometry](docs/preview-recipes.md)
- [Coverage and game verification](docs/coverage.md)
- [Data model and selection identity](docs/data-model.md)
- [Asset flow and publication workflows](docs/workflows.md)
- [Machine-readable coverage evidence](coverage/README.md)

The hosted browser and downloadable offline edition consume prebuilt data and media. Developer tooling uses native Rust libraries: `ghostline-red` reads game resources and `renderer` produces previews. `character-builder` consumes published selections and previews, and assembles live characters through those same libraries.
