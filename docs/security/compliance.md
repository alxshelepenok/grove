---
type: policy
status: stable
---

# Compliance and versioning

This document records Grove's regulatory posture, versioning and support policy, release signing coverage, network egress, and telemetry policy. It is an engineering record of project decisions, not legal advice.

## CRA applicability analysis (decision D-08)

Grove is free and open-source software (AGPL-3.0), developed and distributed by a single developer outside any commercial activity: no sale, no paid support, no donations gate, no monetised distribution channel. The EU Cyber Resilience Act (Regulation (EU) 2024/2847) targets products "made available on the market" in the course of a commercial activity, and its recitals carve out free and open-source software developed and supplied outside commercial activity. On that reading, Grove is very unlikely to fall under the CRA's manufacturer or open-source software steward obligations today.

This is an analysis, not a ruling. The conclusion is revisited when any of the following triggers occurs:

- Any form of monetisation: paid support, sponsorship tiers tied to the software, paid features, or a hosted offering.
- Distribution through a commercial entity or under a dual-licensing arrangement.
- Employment or contracting of the maintainer to work on Grove.
- Publication of ENISA guidance or member-state practice that materially changes the treatment of non-commercial OSS under the CRA.
- Grove reaching 1.0, as a scheduled checkpoint.

Until a trigger fires, Grove keeps the lightweight transparency artifacts described in [annex-vii.md](./annex-vii.md) as good practice, not as CRA conformity documentation.

## Versioning and support policy

Grove is pre-1.0 and follows semantic versioning with the usual pre-1.0 caveat: minor releases may break compatibility. There is a single rolling release line (`stable` channel in `manifest.json`); every release supersedes all earlier ones.

- Only the latest release receives fixes, security or otherwise. There are no backport branches; security fixes ship as patch releases on the current line. This matches the supported-versions table in [SECURITY.md](../../SECURITY.md).
- Older releases receive nothing. The installer enforces forward movement: it refuses manifests with a sequence number lower than the installed one (anti-rollback state in `~/.grove/.sequence`).
- From 1.0 onward the policy extends to the latest plus the previous minor release.

## Release signing coverage

Signatures are RSA-2048/PSS with the dedicated release key; the public half is committed at `docs/security/artifacts/public-keys/grove-manifest-2026-08.pem`. The table reflects what `.github/workflows/release.yml` actually signs and attaches.

| Artifact | Signed | Notes |
| --- | --- | --- |
| `manifest.json` | Yes (`manifest.json.sig`) | Root of trust: version, sequence, expiry, per-artifact SHA-256 and size |
| `SHA256SUMS` | Yes (`SHA256SUMS.sig`) | Hashes of all release binaries and bundles |
| `sbom.cdx.json` | Yes | CycloneDX 1.6 SBOM, mirrored under `docs/security/artifacts/` |
| `vex.json` | Yes | VEX statements, mirrored under `docs/security/artifacts/` |
| `grove-skill.md` | Yes (`grove-skill.md.sig`) | Agent skill bundle |
| `install.sh` / `install.ps1` | Yes (`.sig`) | Re-signed only when the committed copy fails verification |
| CLI / desktop portable `tar.gz` | Indirectly | Covered by the signed manifest hashes and `SHA256SUMS.sig`; GitHub build-provenance attestation attached |
| OS bundles (`msi`, `nsis setup.exe`, `dmg`, `deb`, `AppImage`) | Indirectly | Same coverage as above; no Authenticode signature or Apple notarization |

The OS bundles are unsigned in the publisher-identity sense: Windows SmartScreen and macOS Gatekeeper will warn on browser downloads. This is expected and documented in `docs/install.md`; the signed manifest and `SHA256SUMS.sig` are the verification path.

## Network egress allowlist

The installed binaries (`grove`, `grove-mcp`, `grove-desktop`) make no network calls. `grove-mcp` speaks MCP over stdio only; it does not call LLM provider APIs - the MCP client (Kimi Code, Claude Desktop, ...) owns that traffic. The desktop UI is restricted by CSP (`connect-src 'self' ipc: http://ipc.localhost`).

| Host | Who contacts it | What for |
| --- | --- | --- |
| `raw.githubusercontent.com` | `install.sh`, `install.ps1` | Latest `manifest.json` / `manifest.json.sig` and the installers themselves |
| `github.com` (release downloads) | `install.sh`, `install.ps1`, manual installs | Pinned manifests and release artifacts; artifact URLs are pinned to this host and any other host is rejected |
| `ghcr.io` | CI only (trivy) | Vulnerability database download; never contacted by shipped software |

The installers refuse artifact URLs outside the GitHub release host (see the allowlist check in `install.sh` / `install.ps1`). The `GROVE_ARTIFACT_URL` break-glass mode bypasses all verification by explicit opt-in and is documented as such.

## Telemetry policy

Grove collects no telemetry: no analytics, no crash reporting, no usage metrics, no update check beyond the manifest fetch the user explicitly runs through the installer. There is no telemetry code in any package; a regression here is a bug to report via the security policy.

## Deferred options (decision D-05 context)

Options considered and explicitly deferred; each has a recorded trigger to reopen:

- **Nightly channel**: the manifest format supports channels (`channels.<name>`), but only `stable` is published until there is demonstrated demand.
- **cargo deny / additional scanners**: trivy (pinned, checksum-verified) covers both `Cargo.lock` and `packages/grove/Manifest.toml` in the weekly pipeline and the release gate. Revisit if the dependency graph grows native or transitive-risk-heavy subtrees.
- **PGP key and `security.txt`**: revisit if Grove gets a website of its own; GitHub Security Advisories plus the committed RSA release key suffice for a GitHub-only distribution.
- **Offline / hardware-backed signing**: releases are signed in CI with the key held as an environment secret on the protected `release` environment. Return to offline or hardware-key signing if the project gains contributors with release rights, if the key's blast radius grows (long-lived channels, auto-updaters), or after any CI trust-boundary incident.
