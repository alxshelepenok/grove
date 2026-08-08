---
type: runbook
status: stable
---

# Triaging scan findings into VEX

The `Security scan` workflow (`.github/workflows/security-scan.yml`) runs the trivy audit on every PR, on demand, and weekly (Tuesdays 03:41 UTC). The audit gate fails on any CRITICAL/HIGH finding not covered by a `not_affected` statement in the signed VEX document `docs/security/artifacts/vex.json` (CycloneDX 1.6). This runbook turns a new finding into a triaged statement or a fix. No finding stays untriaged past one weekly cycle.

## Prerequisites

- Repository checkout with Julia available (`julia --project=packages/grove` instantiable).
- trivy 0.73.0 (install exactly as the workflow does, including the checksum verification).
- Familiarity with the VEX schema reference at `docs/security/artifacts/schemas/vex.schema.json`.

## Steps

1. Reproduce the finding locally:
   ```bash
   trivy fs --scanners vuln --format json --output report.json Cargo.lock
   julia --project=packages/grove bin/audit-filter.jl report.json docs/security/artifacts/vex.json
   ```
   Repeat with `packages/grove/Manifest.toml` as the target for Julia findings. Unsuppressed CRITICAL/HIGH entries are printed one per line; MEDIUM/LOW/UNKNOWN appear as non-blocking warnings.
2. Assess the finding. Decide one of:
   - **Fix the dependency**: upgrade the crate or Julia package (`cargo update -p <pkg>` / `Pkg.update`), verify the finding disappears from the trivy report, ship in the next release. No VEX statement is needed once the vulnerable version is gone.
   - **Suppress with VEX**: the project is genuinely not affected. Choose a `justification` from the allowed set: `code_not_present`, `code_not_reachable`, `requires_configuration`, `requires_dependency`, `requires_environment`, `protected_by_compiler`, `protected_at_runtime`, `protected_at_perimeter`, `protected_by_mitigating_control`. If none fits, write a `detail` explaining why - a `not_affected` statement with neither is rejected by the validator.
3. Add the statement to `docs/security/artifacts/vex.json`. Minimal shape:
   ```json
   {
     "id": "CVE-YYYY-NNNNN",
     "affects": [ { "ref": "pkg:cargo/name@1.2.3" } ],
     "analysis": {
       "state": "not_affected",
       "justification": "code_not_reachable",
       "detail": "why the vulnerable code path cannot be reached in grove"
     }
   }
   ```
   Rules enforced by `bin/validate-vex.jl`:
   - `id` must match `CVE-`, `GHSA-`, `JLSEC-`, or `RUSTSEC-` formats.
   - `state` is one of `affected`, `not_affected`, `fixed`, `under_investigation`.
   - `affects[].ref` must be a package URL naming the exact installed version - `bin/audit-filter.jl` suppresses only when both the package name AND the version in the purl match the finding. A bare `pkg:cargo/name` never suppresses.
4. Validate:
   ```bash
   julia --project=packages/grove bin/validate-vex.jl docs/security/artifacts/vex.json
   ```
   Expected: `OK: N statement(s) valid`.
5. Confirm the gate is green end-to-end:
   ```bash
   bin/audit.sh --trivy-bin trivy --fail-closed
   ```
   Expected: `audit ok`.
6. Commit `vex.json` to `main`. The statement takes effect for the audit gate immediately.
7. Refresh the published signature. `vex.json.sig` attached to the current release becomes stale the moment `vex.json` changes. Either:
   - wait for the next release (it re-signs `vex.json` automatically), or
   - re-sign immediately:
     ```bash
     gh workflow run release.yml -f mode=transparency-only
     ```
     Approve the `release` environment gate; the job re-validates `vex.json`, re-signs `vex.json` (and `sbom.cdx.json` when present), uploads both with `--clobber` to the latest release, and commits the transparency log back (merge the `release/transparency-only-<timestamp>` PR if direct push is protected).

## SLA and escalation

- Every new CRITICAL/HIGH finding must be triaged (fixed or covered by a valid `not_affected` statement) before the next weekly scan; the workflow failing on `main` is the enforcement mechanism.
- If the project IS affected and no fix is available, keep the statement at `under_investigation` (which does not suppress the gate) and treat remediation as release-blocking. User-facing impact is handled per `SECURITY.md`, and a signing-pipeline angle per `incident-response.md`.
