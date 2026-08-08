---
type: system
status: stable
---

# Supply chain trust architecture

Describes how Grove controls what third-party code ships: the dependency policy, the trivy-based audit pipeline with VEX suppression, SBOM generation, the weekly scan cadence, vendored JavaScript provenance, and the CI hardening that surrounds all of it.

## Problem

Grove ships Rust binaries built from `Cargo.lock`, a Julia package with `packages/grove/Manifest.toml`, and two vendored JavaScript files in the desktop UI. Each ecosystem has a different lock format and advisory feed. The pipeline must answer two questions on every release and on a fixed cadence: what exactly is in the shipped bytes (SBOM), and are any known-vulnerable components shipping without a documented justification (audit with VEX)?

## Dependency policy

New runtime dependencies require justification, lockfile updates, and a clean audit before merge; the full policy for contributors lives in `CONTRIBUTING.md`. The scanner choice is recorded as decision D-06: trivy runs behind the in-repo policy wrapper `bin/audit.sh` rather than being invoked directly, so severity policy, VEX suppression, and fail-closed behavior are versioned with the code. osv-scanner was ruled out as the primary scanner because it cannot parse Julia `Manifest.toml`; trivy covers both `Cargo.lock` and the Julia manifest with one tool. There is no secondary OSV-query fallback script in the repository.

## Audit pipeline

`bin/audit.sh [--fail-closed] [--trivy-bin trivy] [--vex docs/security/artifacts/vex.json] [--targets ...]`:

1. Runs `trivy fs --scanners vuln --format json` per target; default targets are `Cargo.lock` and `packages/grove/Manifest.toml`.
2. Filters each report through `julia --project=packages/grove bin/audit-filter.jl <report> <vex>`, which drops findings suppressed by the VEX document and classifies the rest: CRITICAL and HIGH are fatal, lower severities are warnings.
3. Exits non-zero on any unsuppressed CRITICAL/HIGH finding. A trivy operational failure (network, DB download) is a warning by default and fatal under `--fail-closed`, so scheduled runs and releases cannot silently pass on a broken scanner while a PR is not blocked by a flaky feed.

VEX suppression is exact: `bin/audit-filter.jl` matches a finding only when the VEX statement id equals the vulnerability id and an `affects[].ref` purl matches the package name and installed version exactly (case-insensitive). Broad or version-less suppressions do not apply.

## VEX workflow

`docs/security/artifacts/vex.json` is a CycloneDX 1.6 VEX subset (currently an empty `vulnerabilities` array). Statements use `analysis.state` of `affected`, `not_affected`, `fixed`, or `under_investigation`; only `not_affected` suppresses audit findings, and every `not_affected` statement must carry a `justification` (from the CycloneDX vocabulary, e.g. `code_not_reachable`) or a non-empty `detail`. Ids must match CVE, GHSA, JLSEC, or RUSTSEC formats.

- `bin/validate-vex.jl` enforces these rules in code (structure, id patterns, states, justifications); the companion JSON Schema for editors and external tooling is `docs/security/artifacts/schemas/vex.schema.json`.
- The VEX document is validated in both `security-scan.yml` and the release `sbom` job, signed with the release key, committed under `docs/security/artifacts/`, and attached to every release as `vex.json(.sig)`, so the suppression basis for any shipped artifact is publicly auditable.

## SBOM generation

`bin/sbom.sh --output sbom.cdx.json` produces a single CycloneDX 1.6 document by merging:

- `cargo-cyclonedx` 0.5.9 (spec 1.5) over the three Rust crates: `packages/core`, `packages/mcp`, `packages/desktop/src-tauri`.
- `trivy fs --format cyclonedx --scanners vuln --list-all-pkgs` over `packages/grove` for the Julia dependency tree.
- A synthesized component list for the vendored JavaScript files, parsed directly out of `packages/desktop/ui/js/vendor/PROVENANCE.md`; `sbom.sh` fails if the provenance table format drifts, so an unrecorded or misrecorded vendored file breaks the build.

`bin/merge-cdx.jl` merges the inputs into CycloneDX 1.6: components are deduplicated by purl (falling back to bom-ref), the `Manifest.toml` pseudo-component is dropped, and components and dependency edges are emitted in sorted order with tool metadata (`cargo-cyclonedx`, `trivy`). The SBOM is signed, committed to `docs/security/artifacts/sbom.cdx.json(.sig)`, and attached to every release.

## Scan cadence

`.github/workflows/security-scan.yml` runs on pull requests, on a weekly schedule (cron `41 3 * * 2`, Tuesdays 03:41 UTC), and on manual dispatch. Each run installs trivy 0.73.0 with checksum verification against the upstream checksums file, instantiates the Julia project, validates the VEX document, runs the audit test suite `tests/audit/test-audit.sh` (fixtures, VEX suppression, fail-open/fail-closed behavior), then runs the audit gate: `--fail-closed` on schedule and dispatch, fail-open on scanner errors for PRs. The release workflow repeats the same gate with `--fail-closed` in its `audit` job, so a release cannot ship with an unsuppressed CRITICAL/HIGH finding or a broken scanner.

## Vendored JavaScript provenance

`packages/desktop/ui/js/vendor/PROVENANCE.md` records every vendored file with its npm package, exact version, upstream URL, and SHA-256; the bytes must be identical to the upstream download and are re-verifiable at any time with `sha256sum`. Current entries: `d3.js` (d3 7.9.0, ISC, used by the graph view `js/views/graph.js`) and `rx.js` (rxjs 7.8.2, Apache-2.0, retained for planned reactive UI work and deliberately not loaded by any page). New vendored files must be recorded with version and hash and keep their upstream license banner; the SBOM treats this file as the source of truth for the JS supply chain.

## CI hardening

- Default `permissions: contents: read` in both workflows; only `sign-and-publish` escalates, and only inside the approval-gated `release` environment.
- All third-party actions are pinned to full commit SHAs; build tools are version-pinned (`cargo * --locked`, trivy and its checksums, `cargo-cyclonedx` 0.5.9, `tauri-cli` 2.11.4, Julia 1.12).
- Concurrency group `grove-release-<mode>` serializes release runs; the signing secret is scoped to the `release` environment and is never exposed to dry-runs (which use an ephemeral in-job key).
- Build jobs in release mode check out the release tag and assert the checkout matches the tag on origin before compiling.

## Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| New CRITICAL/HIGH CVE in a shipped dependency | Weekly fail-closed scan plus a release-time gate; fixes ship on the current release line (see `SECURITY.md`). |
| Scanner outage masks findings | `--fail-closed` on scheduled and release runs turns operational errors into failures; PR runs stay fail-open to avoid blocking on feed flakiness. |
| VEX used to hide real exposure | Exact id plus name plus version matching, mandatory justification or detail, signed and published `vex.json`, validation in CI. |
| Vendored JS silently modified | Provenance table with upstream URL and SHA-256; `sbom.sh` fails if the table cannot be parsed. |
| Malicious or drifting CI tooling | SHA-pinned actions, pinned tool versions, checksum verification for downloaded tools. |

## References

- `bin/audit.sh`, `bin/audit-filter.jl`, `bin/validate-vex.jl`, `bin/sbom.sh`, `bin/merge-cdx.jl`.
- `.github/workflows/security-scan.yml`, `.github/workflows/release.yml`.
- `docs/security/artifacts/vex.json`, `docs/security/artifacts/schemas/vex.schema.json`, `packages/desktop/ui/js/vendor/PROVENANCE.md`.
- `docs/architecture/release-distribution.md`: where the audit and SBOM jobs sit in the release pipeline.
- `SECURITY.md`: reporting, triage, and the role of signed VEX statements in vulnerability handling.
