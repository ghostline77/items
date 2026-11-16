# Items contributor guide

- Read `docs/coverage.md`, `docs/data-model.md` and `docs/workflows.md` before changing catalogue contracts.
- Keep production ingestion, generation and packaging tooling in Rust. Game format knowledge belongs in `ghostline-red`; reusable rendering belongs in `renderer`.
- Verify Cyberpunk-specific claims against local modding documentation and selected game sources. Record provenance and unresolved behavior explicitly.
- Preserve native identities and unknown values. Do not infer compatibility, availability or spawnability from names alone.
- Keep gameplay records, visual assets, customization controls, selections and previews distinct. Report their coverage separately.
- Keep generated game dumps and render intermediates in ignored `work/`; release packages belong in ignored `dist/`.
- Write durable usage and architecture documentation. Do not put progress logs or conversational history in reference docs.
- Keep browsing independent of game installation, extraction tools and online services. Validate both hosted subpath deployment and offline `file://` access when implementing the browser.
