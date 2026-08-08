---
type: runbook
status: stable
---

# Publishing a release

Publish a signed grove release `vX.Y.Z` through the `Release` workflow (`.github/workflows/release.yml`). The workflow builds all targets, runs the audit gate, signs the manifest and transparency artifacts inside the approval-gated `release` environment, publishes the GitHub Release, runs a post-release smoke test against the published assets, and commits the transparency log back to the repository.

## Prerequisites

- Write access to the repository and approver rights on the GitHub environment `release`.
- `gh` CLI authenticated against `alxshelepenok/grove`.
- The release signing key is in place (see `key-generation.md`) and the current manifest on `main` verifies.

## Steps

1. Bump the version in all five version-bearing files on a branch:
   - `packages/core/Cargo.toml`
   - `packages/mcp/Cargo.toml`
   - `packages/desktop/src-tauri/Cargo.toml`
   - `packages/desktop/src-tauri/tauri.conf.json`
   - `packages/grove/Project.toml`

   Open a PR to `main`, get it reviewed and merged.
2. Create and push the annotated tag from the merged commit on `main`:
   ```bash
   git checkout main && git pull
   git tag -a vX.Y.Z -m "vX.Y.Z"
   git push origin vX.Y.Z
   ```
3. Rehearse with a dry run (uses an ephemeral in-job key and the `release-dry` environment; nothing is published):
   ```bash
   gh workflow run release.yml -f mode=dry-run -f version=X.Y.Z
   ```
4. Dispatch the real release:
   ```bash
   gh workflow run release.yml -f mode=release -f version=X.Y.Z
   ```
5. Watch the run and approve the `release` environment gate when the `sign-and-publish` job pauses:
   ```bash
   gh run list --workflow=release.yml --limit 1
   ```
   Open the run in the Actions UI, click "Review deployments", approve `release`. The workflow checks out `refs/tags/vX.Y.Z` and asserts HEAD matches the tag, so a missing or moved tag fails here.
6. Verify the "Post-release smoke test" step of the `sign-and-publish` job. It downloads the published `manifest.json`, `install.sh`, and `install.ps1` with their signatures, verifies them against the committed public key, and hashes the published `grove-mcp` and `grove-desktop` linux-x64 archives against the manifest. Expected output ends with:
   ```text
   smoke ok: published manifest, install.sh, install.ps1 verify; grove-mcp and grove-desktop linux-x64 match their signed hashes
   ```
7. Merge the commit-back PR. The "Commit transparency log" step pushes `manifest.json`, `manifest.json.sig`, `docs/security/artifacts/`, `install.sh.sig`, and `install.ps1.sig` to `main`; when direct push is protected it opens a PR from branch `release/vX.Y.Z` titled "release: transparency artifacts (release X.Y.Z)":
   ```bash
   gh pr list --head release/vX.Y.Z
   gh pr merge release/vX.Y.Z --merge
   ```
8. Run the per-release manual security checks from `release-security-checks.md`.

## Rollback

If the release is bad (wrong artifacts, failed smoke after publish, wrong version):

1. Delete the GitHub Release and its assets:
   ```bash
   gh release delete vX.Y.Z --yes
   ```
2. Delete the tag remotely and locally:
   ```bash
   git push origin --delete vX.Y.Z
   git tag -d vX.Y.Z
   ```
3. Fix the cause, re-tag, and re-dispatch from step 3. The release preconditions in the workflow refuse to run while a release for the tag still exists, so step 1 is mandatory before re-running.
4. The manifest `sequence` is monotonic: a re-run consumes the next sequence number. This is expected and harmless; never re-use a lower sequence, because installed clients reject it as a rollback (`~/.grove/.sequence`).
