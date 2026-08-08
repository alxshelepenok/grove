---
type: runbook
status: stable
---

# Refreshing an expiring manifest

The root `manifest.json` carries `expires_at` set to `created_at` plus 180 days (`--ttl-days 180` default in `bin/manifest.sh`). Both installers reject a manifest one day after `expires_at` ("Manifest expired. A new release is pending; try again later."). If more than 180 days pass without a release, installs start failing. This runbook re-signs a refreshed manifest without cutting a release, using the `manifest-only` mode of the `Release` workflow.

## Prerequisites

- Approver rights on the GitHub environment `release`; `gh` authenticated.
- At least one published release exists (the refreshed manifest is attached to the latest one).

## Trigger check

Confirm the manifest is expiring or expired:

```bash
grep -E '"(created_at|expires_at)"' manifest.json
date -u +%Y-%m-%dT%H:%M:%SZ
```

Act when `expires_at` is less than 30 days away, or immediately once installs report expiry.

## Steps

1. Dispatch the workflow in `manifest-only` mode (no `version` input):
   ```bash
   gh workflow run release.yml -f mode=manifest-only
   ```
2. Approve the `release` environment gate when the `sign-and-publish` job pauses ("Review deployments" in the Actions UI).
3. The job then runs, on a checkout of `main`:
   ```bash
   bin/manifest.sh --refresh --previous manifest.json --output manifest.json
   bin/sign.sh key.pem manifest.json
   bin/verify.sh trusted.pem manifest.json manifest.json.sig
   gh release upload "$tag" manifest.json manifest.json.sig --clobber
   ```
   `--refresh` bumps the channel `sequence` by one and sets a new `created_at`/`expires_at` (+180 days), leaving every artifact entry untouched. The refreshed files are uploaded to the latest release with `--clobber`.
4. Merge the commit-back PR if direct push to `main` is protected. Without a version input the branch is `release/manifest-only-<timestamp>`:
   ```bash
   gh pr list --search "release: transparency artifacts (manifest-only" --state open
   ```

## Verification

Download the refreshed manifest from the latest release and verify it against the committed public key:

```bash
tag=$(gh release list --limit 1 --json tagName --jq '.[0].tagName')
mkdir -p /tmp/grove-refresh && cd /tmp/grove-refresh
curl -fsSL -O "https://github.com/alxshelepenok/grove/releases/download/$tag/manifest.json" \
            -O "https://github.com/alxshelepenok/grove/releases/download/$tag/manifest.json.sig"
bin/verify.sh <repo>/docs/security/artifacts/public-keys/grove-manifest-2026-08.pem manifest.json manifest.json.sig
grep -E '"(created_at|expires_at)"' manifest.json
sed -n 's/^      "sequence": \([0-9][0-9]*\),$/\1/p' manifest.json
```

Expected: `OK` from `bin/verify.sh`, `expires_at` ~180 days in the future, `sequence` exactly one higher than before. Optionally confirm a fresh install succeeds:

```bash
bash install.sh --self-test
```

## Rollback

A refresh only extends validity and bumps the sequence; there is nothing to roll back. If the refreshed manifest is wrong (bad signature, wrong sequence), re-run the dispatch - each run consumes one more sequence number, which is harmless. Never hand-edit `manifest.json` to lower the sequence: installed clients reject it as a rollback.
