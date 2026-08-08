---
type: policy
status: stable
---

# Annex VII technical documentation

This document records, at a high level, what CRA Annex VII (technical documentation under Regulation (EU) 2024/2847, Article 31) would require, why a full Annex VII conformity package is not applicable to Grove today, and the transparency artifact inventory Grove maintains as a lightweight analogue. It is an engineering record, not legal advice.

## What Annex VII would require

For a product placed on the EU market, the manufacturer must draw up technical documentation demonstrating conformity with the essential cybersecurity requirements of Annex I. At a high level the documentation set covers:

- A general description of the product: intended purpose, versions, architecture, and how it is made available.
- Design and development information: how security is built in, including the handling of dependencies and components, and a software bill of materials.
- Vulnerability handling: processes for receiving, assessing, and remediating vulnerabilities, and for coordinated disclosure.
- The risk assessment against the Annex I requirements and how each applicable requirement is met.
- Test reports and evidence supporting conformity, plus the applied harmonised standards or other specifications.
- A declaration of how the support period is set and how end-of-life is handled.

The documentation must exist before the product is placed on the market, be kept up to date during the support period, and be retained for at least 10 years (or the support period, if longer), available to market surveillance authorities on request.

## Why full Annex VII conformity does not apply to Grove today

Per the analysis in [compliance.md](./compliance.md) (decision D-08), Grove is free and open-source software developed and distributed outside commercial activity, so the CRA's manufacturer obligations - including the Article 31 technical documentation duty - are very unlikely to apply. There is no conformity assessment, no EU declaration of conformity, and no CE marking for Grove.

The revisit triggers recorded in `compliance.md` (monetisation, commercial distribution, changed OSS guidance, or the 1.0 checkpoint) would reopen this document; if any fires, this inventory becomes the seed of a real Annex VII package rather than a substitute for one.

## Transparency artifact inventory

Grove maintains the following artifacts as good security practice. They map loosely onto Annex VII themes (SBOM, vulnerability handling, release integrity) without claiming conformity.

| Artifact | Location | Signed | Purpose |
| --- | --- | --- | --- |
| Release manifest | `manifest.json` + `manifest.json.sig` (repo root) | Yes | Version, channel sequence (anti-rollback), expiry, per-artifact URL / SHA-256 / size; root of trust for installers |
| SBOM | `docs/security/artifacts/sbom.cdx.json` + `.sig` | Yes | CycloneDX 1.6 bill of materials covering the Rust workspace and the Julia project |
| VEX statements | `docs/security/artifacts/vex.json` + `.sig` | Yes | Triage of third-party dependency findings; `not_affected` statements are honored by the audit gate |
| Release public key | `docs/security/artifacts/public-keys/grove-manifest-2026-08.pem` | - | Public half of the RSA-2048/PSS release signing key; fingerprint recorded in the key-generation runbook |
| JSON schemas | `docs/security/artifacts/schemas/` | - | Machine-checkable contracts for the transparency artifacts (currently the VEX schema, enforced by `bin/validate-vex.jl` in CI) |
| Runbooks | `docs/security/runbooks/` | - | Procedures and execution logs for key lifecycle and related operations |

Vulnerability handling itself is documented in [SECURITY.md](../../SECURITY.md) (reporting channels, response targets, supported versions), and the dependency audit gate (`bin/audit.sh`, pinned trivy, fail-closed in release and weekly CI) enforces the findings pipeline the VEX file feeds.

## Retention policy

- All artifacts above live in git: history is append-only in practice, so every past version of the manifest, SBOM, VEX, schemas, and runbooks remains retrievable by commit. In-git history is the retention mechanism.
- Artifacts are signed at release time with the release key (RSA-2048/PSS), and each release attaches the current set (`manifest.json(.sig)`, `SHA256SUMS(.sig)`, SBOM, VEX, signed installers) to its GitHub Release, giving a per-release immutable snapshot outside the repository as well.
- The release workflow commits the refreshed transparency artifacts back to `main` after each publish, so the repo state and the release attachments stay in sync. Committing a signed artifact invalidates nothing: signatures are over file bytes, and the bytes are preserved verbatim in git.
- There is no deletion schedule. If the CRA triggers in `compliance.md` ever fire, a formal retention clock (10 years or support period, whichever is longer) starts from that point and this policy is revisited.
- The release workflow may re-sign and re-attach refreshed transparency artifacts to the latest release (`manifest-only` and `transparency-only` modes); the git history remains the canonical, non-rewritten record of every earlier version.
