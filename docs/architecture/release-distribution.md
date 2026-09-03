---
type: system
status: stable
---

# Release distribution architecture

Describes how Grove is built, signed, and distributed: the GitHub Releases channel, the `release.yml` workflow anatomy, the artifact inventory attached to every release, the signed manifest schema, and the commit-back flow that keeps the repository root manifest in sync.

## Problem

Grove ships pre-built binaries (Rust CLI, MCP server, Tauri desktop) plus a Julia source tree, and users install them by piping a script from the repository into a shell. Without a signed manifest and a disciplined pipeline, a compromised release asset or a swapped installer would be indistinguishable from a legitimate one. The pipeline must produce, sign, and publish artifacts in one auditable run, with the signing step isolated behind a manual approval gate.

## Distribution channel

GitHub Releases on `alxshelepenok/grove` is the only distribution channel (decision D-09). There is no package registry, no auto-updater, and no separate download host.

- Release assets (archives, bundles, manifest, signatures, SBOM, VEX) are attached to the tag release `vX.Y.Z`.
- The installers `install.sh` and `install.ps1` are served from `raw.githubusercontent.com` on the `main` branch; detached signatures (`install.sh.sig`, `install.ps1.sig`) are attached to every release for out-of-band verification.
- The rolling `manifest.json` at the repository root is the default pointer for installs; `--version X.Y.Z` pins to the manifest attached to that release instead.

## Release workflow anatomy

`.github/workflows/release.yml` is dispatch-only (`workflow_dispatch`) with two inputs: `version` (`X.Y.Z`, required for release and dry-run) and `mode` (`dry-run` default, `release`, `manifest-only`, `transparency-only`). A concurrency group `grove-release-<mode>` with `cancel-in-progress: false` serializes runs per mode. Default permissions are `contents: read`; only the publish job requests more.

Jobs:

| Job | Runs when | Purpose |
| --- | --- | --- |
| `validate` | release, dry-run | Asserts `version` matches `^[0-9]+\.[0-9]+\.[0-9]+$`. |
| `build` | release, dry-run | Matrix over `ubuntu-latest`, `macos-latest`, `macos-15-intel`, `windows-latest`. Builds `grove-core` and `grove-mcp` with `cargo build --release --locked`, runs the workspace tests, packages one `tar.gz` per component per target. |
| `build-desktop` | release, dry-run | Same matrix. Installs `tauri-cli` 2.11.4, runs `cargo tauri build --bundles <per-os>` in `packages/desktop/src-tauri`, runs the desktop test suite, packages the portable `tar.gz` (binary plus `ui/views` and `ui/icons`) plus OS bundles. |
| `audit` | release, dry-run | Dependency audit gate: `bin/audit.sh --trivy-bin trivy --fail-closed` (see `supply-chain-trust.md`). |
| `sbom` | release, dry-run | Generates `sbom.cdx.json` via `bin/sbom.sh` and validates `docs/security/artifacts/vex.json` via `bin/validate-vex.jl`. |
| `sign-and-publish` | all modes, gated on `!failure()` | Signs, publishes, smoke-tests, and commits back transparency artifacts. |

In `release` mode every build job checks out `refs/tags/v<version>` and asserts the checkout matches the tag on origin (`Assert checkout matches the tag`). The tag must exist before dispatch; `gh release create` runs with `--verify-tag`.

### Modes

- `dry-run`: full build, audit, SBOM, and signing exercise, but with an ephemeral RSA key generated inside the job; the real secret is never touched. Runs in the `release-dry` environment. Nothing is published or committed.
- `release`: full pipeline in the approval-gated `release` environment. Publishes the GitHub release, attests provenance, smoke-tests the published assets, and commits transparency artifacts back.
- `manifest-only`: refreshes `created_at` / `expires_at` and bumps `sequence` via `bin/manifest.sh --refresh`, re-signs, and uploads `manifest.json(.sig)` to the latest release with `--clobber`. Used to extend manifest validity between binary releases.
- `transparency-only`: re-signs `vex.json` and `sbom.cdx.json` under `docs/security/artifacts/` and uploads them (plus signatures) to the latest release.

### Signing and publish gate

`sign-and-publish` runs in GitHub environment `release` (`release-dry` for dry-run). The `release` environment is approval-protected and holds the `GROVE_MANIFEST_SIGNING_KEY` secret, so no signing happens without a human approval. The job alone gets `contents: write`, `id-token: write`, `attestations: write`, and `pull-requests: write`. Release-mode preconditions re-validate the version, assert the tag exists, and refuse if the release already exists.

All third-party actions are pinned to full commit SHAs. Build tooling is version-pinned (`tauri-cli` 2.11.4, `cargo-cyclonedx` 0.5.9, trivy 0.73.0 with checksum verification against the upstream `trivy_0.73.0_checksums.txt`, Julia 1.12).

## Artifact inventory per release

`gh release create v<version>` attaches, for each of the four targets `linux-x64`, `macos-arm64`, `macos-x64`, `windows-x64`:

- CLI archives: `grove-v<version>-<target>.tar.gz`, `grove-mcp-v<version>-<target>.tar.gz` (8 total).
- Desktop portable archives: `grove-desktop-v<version>-<target>.tar.gz` (4 total).
- Desktop OS bundles: `grove-desktop-v<version>-windows-x64-setup.exe` (NSIS) and `.msi`; `grove-desktop-v<version>-macos-arm64.dmg` and `-macos-x64.dmg`; `grove-desktop-v<version>-linux-x64.deb` and `.AppImage`. These bundles are unsigned: browser downloads trigger SmartScreen / Gatekeeper warnings; the signed manifest and `SHA256SUMS.sig` are the verification path.

Plus the trust and transparency files:

- `SHA256SUMS` and `SHA256SUMS.sig` over every archive.
- `manifest.json` and `manifest.json.sig`.
- `sbom.cdx.json` and `sbom.cdx.json.sig` (CycloneDX 1.6).
- `vex.json` and `vex.json.sig`.
- `install.sh`, `install.sh.sig`, `install.ps1`, `install.ps1.sig`. Installer signatures are only re-created when they no longer verify against the current key (key rotation), otherwise the committed `.sig` files ship as-is.
- `grove-skill.md` and `grove-skill.md.sig`: the single-file agent skill bundle from `bin/skill-bundle.sh`.
- SLSA build provenance attestations via `actions/attest-build-provenance` covering all archives, bundles, and `manifest.json`.

All `.sig` files are detached RSA-2048/PSS over SHA-256, base64url without padding (see `bin/sign.sh`).

## Manifest schema

`manifest.json` (generated by `bin/manifest.sh`, example at the repository root):

| Field | Type | Meaning |
| --- | --- | --- |
| `version` | string `X.Y.Z` | Release version this manifest points at. |
| `created_at` | string (ISO 8601 UTC) | Generation time. |
| `expires_at` | string (ISO 8601 UTC) | Staleness deadline; default TTL is 180 days (`--ttl-days 180`). Installers fail closed 24 hours past this instant. |
| `channels` | object | Map of channel name (`^[a-z0-9_-]+$`) to channel payload. Only `stable` exists today. |
| `channels.stable.sequence` | integer | Monotonic anti-rollback counter. `bin/manifest.sh` sets it to `previous + 1` by reading `--previous manifest.json`. |
| `channels.stable.artifacts` | object | Map of `<component>_<target>` keys (dashes normalized to underscores, e.g. `grove_mcp_linux_x64`) to descriptors. |

Each artifact descriptor: `url` (pinned to `https://github.com/alxshelepenok/grove/releases/download/v<version>/<file>`), `sha256` (hex), `size` (bytes). Only the twelve portable `tar.gz` archives appear in the manifest; OS bundles are covered by `SHA256SUMS.sig` instead.

The signature covers the exact bytes of `manifest.json`; installers verify before parsing (see `bootstrap-trust.md`).

## Versioning and commit-back flow

Releases are tag-driven: the maintainer pushes `vX.Y.Z`, then dispatches `release` mode with that version. Version tags are the only source of release numbering; the workflow never creates tags.

After publishing, the `Commit transparency log` step (skipped for dry-run) stages `manifest.json`, `manifest.json.sig`, `install.sh.sig`, `install.ps1.sig`, and `docs/security/artifacts/`, commits as `grove-compass[bot]`, and pushes to `main`. When branch protection rejects the direct push, the step pushes to `release/v<version>` (or `release/<mode>-<timestamp>` when no version applies) and opens a PR to `main` titled `release: transparency artifacts (...)`. This keeps the root manifest that installers fetch from `raw.githubusercontent.com/.../main` in sync with the latest release.

## Post-release smoke test

In `release` mode the workflow re-downloads the published `manifest.json(.sig)` and both installers with their signatures, verifies all signatures against the committed public key, then downloads `grove-mcp` and `grove-desktop` for `linux-x64` and asserts their SHA-256 equals the value in the signed manifest. A mismatch fails the workflow after publication, surfacing a broken release immediately.

## References

- `docs/architecture/bootstrap-trust.md`: installer-side verification, anti-rollback, break-glass.
- `docs/architecture/supply-chain-trust.md`: audit gate, SBOM, VEX, vendored JS provenance.
- `docs/architecture/key-management.md`: the manifest signing key, storage, and rotation.
- `.github/workflows/release.yml`, `bin/manifest.sh`, `bin/sign.sh`, `bin/verify.sh`.
- `docs/install.md`: user-facing install and verification instructions.
