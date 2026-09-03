---
type: runbook
status: stable
---

# Incident response: signing-key or pipeline compromise

On-call runbook for suspected or confirmed compromise of the grove release signing key (`GROVE_MANIFEST_SIGNING_KEY`) or of the release pipeline itself (GitHub account/token, workflow tampering). Grove signs in CI inside the approval-gated `release` environment; there is no offline signing host to pull, so containment means revoking the secret and freezing dispatches.

## Triggering criteria

Activate this runbook on any of:

- Evidence of a signature or manifest produced outside an approved `sign-and-publish` run (a manifest/signature pair that verifies but matches no release workflow run).
- The `GROVE_MANIFEST_SIGNING_KEY` secret value appearing anywhere outside GitHub Secrets and the offline backup (logs, screenshots, paste sites, a leaked backup medium).
- Offline backup medium lost, stolen, or found with broken tamper evidence.
- An unexpected `release.yml` run, an environment approval you did not grant, a release or tag you did not create, or a commit-back PR with unexpected content.
- Maintainer GitHub account or PAT suspected compromised.

A false-alarm review can follow; do NOT postpone the 1-hour actions pending investigation.

## Timeline

### Within 1 hour - revoke

1. Remove the signing secret from the `release` environment so no further run can sign:
   ```bash
   gh secret delete GROVE_MANIFEST_SIGNING_KEY --env release
   ```
2. Freeze the pipeline: cancel any in-flight release runs and disable the workflow:
   ```bash
   gh run list --workflow=release.yml --status in_progress --json databaseId --jq '.[].databaseId'
   gh run cancel <run-id>
   gh workflow disable release.yml
   ```
3. Preserve evidence before anything ages out: export the run list and the suspicious runs' logs:
   ```bash
   gh api repos/alxshelepenok/grove/actions/runs --jq '.workflow_runs[] | {id, name, event, head_branch, conclusion, created_at}'
   gh run view <run-id> --log > incident-<run-id>.log
   ```

### Within 24 hours - rotate

1. Generate a replacement keypair and rotate per `rotate-signing-key.md`, "Suspected compromise additions": new keypair (`grove-manifest-YYYY-MM`), public key under `docs/security/artifacts/public-keys/`, installers updated, new `GROVE_MANIFEST_SIGNING_KEY` secret.
2. Bump the anti-rollback floor: set `MINIMUM_SEQUENCE` in `install.sh` and `$MinimumSequence` in `install.ps1` to the sequence of the first manifest signed by the new key. This makes every manifest signed by the old key unacceptable to updated installers, regardless of its sequence.
3. Ship an emergency release (or at minimum a `manifest-only` refresh signed by the new key) per `publish-release.md` / `manifest-refresh.md`, then re-enable the workflow:
   ```bash
   gh workflow enable release.yml
   ```

### Within 72 hours - assess and notify

1. Assess the blast radius from the transparency log and release history:
   ```bash
   git log --follow --format='%h %ad %s' --date=iso -- manifest.json
   gh release list --limit 30
   gh release view vX.Y.Z --json tagName,assets,createdAt
   ```
   Diff each `manifest.json` revision against its predecessor: unexpected sequence jumps, changed artifact hashes or URLs, and commits outside the `grove-compass[bot]` commit-back flow mark the compromised window.
2. For every release inside that window, re-verify the published manifest and artifacts with `bin/verify.sh` and the checks in `release-security-checks.md`; flag any release whose assets fail verification for deletion.
3. Notify users via a GitHub Security Advisory on the repository, following the disclosure process in `SECURITY.md`: describe the compromise window, the new key fingerprint, the new `MINIMUM_SEQUENCE`, and instruct users to reinstall with the current `install.sh`/`install.ps1`.

### Within 14 days - review

1. Post-incident review: root cause, timeline, what the transparency log did and did not catch, and concrete hardening actions (e.g. stricter environment protection rules, shorter secret lifetime).
2. Update this runbook, `rotate-signing-key.md`, and `key-generation.md` with the lessons; record the incident in the `key-generation.md` execution log.

## Variant: pipeline compromise without key exposure

When the vector is a malicious workflow run or a stolen GitHub token rather than the key itself:

1. Rotate the GitHub credentials first: revoke the affected PATs/SSH keys, reset the account password, re-enroll 2FA, and sign out all sessions.
2. Audit the environment approvals: in repository Settings -> Environments -> `release`, review the protection rules and confirm every approval on recent `sign-and-publish` deployments was granted by you; tighten required reviewers if anything is ambiguous.
3. Review recent workflow runs and commit-back PRs for tampering (steps 1.3 and 3.1 above); treat any artifact published by a rogue run as compromised and roll it back per `publish-release.md`.
4. The signing key can remain in place only if there is positive evidence the rogue run never reached the "Prepare signing key" step (the step prints the key fingerprint's SHA-256 prefixes to the log - compare them against the value recorded in `key-generation.md`). When in doubt, rotate the key anyway; a scheduled rotation costs one release cycle, a missed compromise costs the trust anchor.
