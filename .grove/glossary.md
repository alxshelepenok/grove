# Glossary

| Term | Definition | Source |
| --- | --- | --- |
| normalization | Masking volatile values (timestamps, checksums, session tokens, machine-specific paths) at capture time so fixtures stay byte-stable across runs and machines | D-01 |
| causal inversion | A dependency edge whose required-by direction opposes the information flow, e.g. an assumption targeting the task whose purpose is to validate that assumption | W-02 |
| thin adapter | An integration surface (MCP server, desktop bridge) that maps its native operations 1:1 onto the core CLI contract and inherits core guards verbatim instead of reimplementing semantics | Y-03 |
| verify-then-parse | A signed artifact (manifest, bootstrap script) is never parsed before its detached signature verifies against the embedded public key; every check fails closed | Y-04 |
| pinned origin | A hard-coded HTTPS host allowlist the installer trusts (GitHub Releases + raw.githubusercontent.com); artifact URLs on any other host are rejected | Y-04 |
| dsh | The DeepSeek Harness CLI and agent runtime; everything in it (models, tools, sessions, UI) is a swappable plugin | D-12 |
| Cordis | The plugin microkernel under dsh; a plugin is a TS module exporting apply(ctx) whose registrations roll back on unload | D-12 |
| bundle | The dsh distribution format: an npm package with a dsh.bundle.patch entry pointing at a cordis.patch.yml that mounts the plugin into a profile | D-12 |
| url-plain asset | An embedded asset filename kept free of characters that need URL percent-encoding, because webviews decode such paths inconsistently across platforms | Y-08 |
| uname shim | A fake `uname` executable prepended to PATH that reports a foreign OS and architecture, so cross-platform shell code takes the target branch during a local run | Q-06 |
| Neo4j | A graph database Grove can mirror into as a read-only Cypher projection for ad-hoc analytics and cross-project federation; never the authoritative store | Q-07 |
| DOM-free module | A JS module with no DOM, WebGL, or global dependency that carries pure interaction math, so a plain test runner can exercise it outside a browser | D-17 |
| vendored ESM pair | Two vendored files where the module entry imports its sibling by a relative specifier, keeping a library loadable without an import map; each file carries its own provenance row and the SBOM deduplicates them by purl | D-18 |
| served-DOM harness | A throwaway HTTP server that mounts the real ui tree behind a minimal DOM skeleton, so the actual view module runs in a real browser and interaction bugs surface as page errors and observable DOM state | W-128 |
| causality cone | The bounded dependency neighborhood of a work item: backward blocks-predecessors in contraction order, forward blocks-successors as impact, per-goal fragility and relevant discoveries | D-20 |
| rendered contract | The observable outputs a test may pin: rendered HTML markup, view-model data, pure-function results - never source text, identifiers, or stylesheet internals, which rot on every refactor | W-165 |
| beam geometry | Real box instances stretched between node centers to draw a 3D edge thicker than the WebGL one-pixel linewidth cap; thickness is set in world units tuned against the default camera radius | W-168 |
| label fade | One shared pure curve mapping zoom-out factor to text opacity (smoothstep from full to ghost to gone) fed per frame into every label system, instead of per-view visibility thresholds | W-170 |
