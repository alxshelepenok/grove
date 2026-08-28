# Agents

This document contains links to available skill documentation for agents.

## Skills

- **[Grove](docs/skills/index.md)**: a dual-track, evidence-based workflow for AI agents.

## Architecture

- **[Bootstrap trust](docs/architecture/bootstrap-trust.md)**: how a fresh user establishes trust in Grove binaries: HTTPS fetch, embedded release public key anchor, and signed-manifest verification of every artifact.
- **[Key management](docs/architecture/key-management.md)**: cryptographic keys behind Grove's release trust: inventory, where each half lives, rotation, and rejected alternatives.
- **[Release distribution](docs/architecture/release-distribution.md)**: build, sign, and distribute flow: GitHub Releases channel, `release.yml` workflow, artifact inventory, signed manifest schema, and commit-back sync.
- **[Supply chain trust](docs/architecture/supply-chain-trust.md)**: third-party code controls: dependency policy, trivy audit pipeline with VEX suppression, SBOM generation, weekly scans, vendored JS provenance, and CI hardening.

## Policy

- **[Annex VII technical documentation](docs/security/annex-vii.md)**: CRA Annex VII (Regulation (EU) 2024/2847, Article 31) applicability assessment and the transparency artifact inventory Grove maintains as a lightweight analogue.
- **[Compliance and versioning](docs/security/compliance.md)**: regulatory posture, versioning and support policy, release signing coverage, network egress, and telemetry policy.

## Runbooks

- **[Publish release](docs/security/runbooks/publish-release.md)**: publish a signed release `vX.Y.Z` through the `Release` workflow, including post-release smoke test and transparency-log commit-back.
- **[Manual release security checks](docs/security/runbooks/release-security-checks.md)**: per-release checklist to run after the commit-back PR merges, before the release is announced.
- **[Release signing key generation](docs/security/runbooks/key-generation.md)**: generate the RSA-2048 release signing keypair, publish the public half, and store the private half in the GitHub `release` environment secret plus offline backup.
- **[Rotate signing key](docs/security/runbooks/rotate-signing-key.md)**: replace the release signing keypair on schedule or after compromise, with an overlap window so old installers still verify.
- **[Refresh manifest](docs/security/runbooks/manifest-refresh.md)**: re-sign the root `manifest.json` when its 180-day TTL nears expiry, without cutting a release.
- **[Update VEX](docs/security/runbooks/update-vex.md)**: triage new trivy findings into `not_affected` statements in the signed VEX document or a fix, within one weekly cycle.
- **[Incident response](docs/security/runbooks/incident-response.md)**: on-call procedure for signing-key or release-pipeline compromise: secret revocation and dispatch freeze in CI-based signing.

## Usage

Each skill document contains:
- Name and description for agent identification.
- Detailed implementation guidelines.
- Code examples and best practices.
- Usage scenarios and recommendations.

Agents should reference these documents when working on related tasks to ensure consistency and follow established patterns.
