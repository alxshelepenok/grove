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
