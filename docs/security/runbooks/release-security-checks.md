---
type: runbook
status: stable
---

# Manual security checks per release

Run this checklist after every published release (after the commit-back PR merged, per `publish-release.md`). Every check must pass before the release is announced. Commands assume a bash shell, repository checkout at `<repo>`, and release version `X.Y.Z`.

## Prerequisites

- `gh`, `curl`, `openssl` in PATH.
- The trusted public key `docs/security/artifacts/public-keys/grove-manifest-2026-08.pem` (or the current key per `rotate-signing-key.md`).

## Verification checklist

Setup shared by all checks:

```bash
mkdir -p /tmp/grove-check && cd /tmp/grove-check
base="https://github.com/alxshelepenok/grove/releases/download/vX.Y.Z"
pub=<repo>/docs/security/artifacts/public-keys/grove-manifest-2026-08.pem
```

### 1. Manifest signature verification

**Purpose:** Ensure the published manifest is signed by the trusted release key (RSA-2048 / RSA-PSS / SHA-256).

**Steps:**
```bash
curl -fsSL -O "$base/manifest.json" -O "$base/manifest.json.sig"
<repo>/bin/verify.sh "$pub" manifest.json manifest.json.sig
```

**Expected Result:** `bin/verify.sh` prints `OK` and exits 0.

### 2. Artifact hash and size match against the manifest

**Purpose:** Ensure every published archive matches the SHA-256 and byte size recorded in the signed manifest.

**Steps:**
```bash
curl -fsSL -O "$base/SHA256SUMS" -O "$base/SHA256SUMS.sig"
<repo>/bin/verify.sh "$pub" SHA256SUMS SHA256SUMS.sig
for f in $(cut -d' ' -f3 SHA256SUMS); do curl -fsSL -O "$base/$f"; done
sha256sum -c SHA256SUMS
```
Then compare each archive's byte size (`stat -c %s <file>`) against the `size` field of its entry in `manifest.json`.

**Expected Result:** Signature `OK`; `sha256sum -c` reports `OK` for every file; every size matches the manifest exactly.

### 3. Installer signatures present and valid

**Purpose:** Ensure `install.sh` and `install.ps1` ship detached signatures made by the current key, so the copies users pipe into a shell match the signed release assets.

**Steps:**
```bash
for f in install.sh install.ps1; do
  curl -fsSL -O "$base/$f" -O "$base/$f.sig"
  <repo>/bin/verify.sh "$pub" "$f" "$f.sig"
done
```

**Expected Result:** Both verifications print `OK`. (The workflow only re-signs an installer when its existing signature fails, so `OK` also confirms the content is the signed one.)

### 4. Transparency artifact signatures

**Purpose:** Ensure the CycloneDX SBOM and VEX document attached to the release verify against the trusted key and match the committed copies.

**Steps:**
```bash
for f in sbom.cdx.json vex.json; do
  curl -fsSL -O "$base/$f" -O "$base/$f.sig"
  <repo>/bin/verify.sh "$pub" "$f" "$f.sig"
  diff "$f" "<repo>/docs/security/artifacts/$f"
done
```

**Expected Result:** Both print `OK`; both `diff`s are empty.

### 5. Sequence number incremented

**Purpose:** Ensure the anti-rollback counter moved forward so installed clients accept the new manifest.

**Steps:**
```bash
sed -n 's/^      "sequence": \([0-9][0-9]*\),$/\1/p' manifest.json
git -C <repo> show 'HEAD~1:manifest.json' | sed -n 's/^      "sequence": \([0-9][0-9]*\),$/\1/p'
```

**Expected Result:** New sequence is exactly previous + 1 (or higher after a re-release), never lower, and >= `MINIMUM_SEQUENCE` in `install.sh` / `$MinimumSequence` in `install.ps1`.

### 6. Manifest expiry window

**Purpose:** Ensure `expires_at` is ~180 days out (the `--ttl-days 180` default in `bin/manifest.sh`), so installs are not rejected as stale before the next release cadence.

**Steps:**
```bash
grep -E '"(created_at|expires_at)"' manifest.json
```

**Expected Result:** `expires_at` minus `created_at` is 180 days, and `expires_at` is comfortably in the future. The installer rejects a manifest one day after `expires_at`; a short window is a latent outage (see `manifest-refresh.md`).

### 7. Artifact download spot-check

**Purpose:** Prove end-to-end that a user-facing download URL from the signed manifest serves the exact signed bytes.

**Steps:**
```bash
curl -fsSL -o spot.tar.gz "$base/grove-mcp-vX.Y.Z-linux-x64.tar.gz"
sha256sum spot.tar.gz
sed -n '/^        "grove_mcp_linux_x64": {$/,/^        }/p' manifest.json
```

**Expected Result:** The computed hash equals the `sha256` of `grove_mcp_linux_x64` in the manifest. Repeat for one windows-x64 archive when the release changes Windows-specific code.

## Recording

Record date, release version, checker, and observed sequence/hash values in the release tracking issue. Any failed check blocks the announcement; investigate, then retry or roll back per `publish-release.md`.
