---
type: runbook
status: stable
---

# Release signing key generation

Generate the dedicated RSA-2048 release signing keypair, publish the public half, and store the private half in the GitHub `release` environment secret and an offline backup. The document is both procedure and record: the execution log at the bottom records what was actually done.

## Prerequisites

- `openssl` 3.x (or LibreSSL 3.3+) on the maintainer machine, bash shell.
- Write access to the GitHub repository settings (environments and secrets).
- An offline backup medium (encrypted USB drive or paper in a safe).
- The GitHub environment `release` must already exist (see repository hardening settings).

## Steps

1. Generate the keypair in a private staging directory (never inside the repository):
   ```bash
   mkdir -p /path/to/staging && chmod 700 /path/to/staging && cd /path/to/staging
   openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out grove-manifest-YYYY-MM.pem
   openssl pkey -in grove-manifest-YYYY-MM.pem -pubout -out grove-manifest-YYYY-MM.pub.pem
   chmod 600 grove-manifest-YYYY-MM.pem
   ```
2. Compute and record the key fingerprint (SPKI SHA-256, first 16 hex chars):
   ```bash
   openssl rsa -in grove-manifest-YYYY-MM.pem -pubout -outform DER 2>/dev/null | sha256sum | cut -c1-16
   ```
3. Commit the public half:
   ```bash
   cp grove-manifest-YYYY-MM.pub.pem <repo>/docs/security/artifacts/public-keys/grove-manifest-YYYY-MM.pem
   ```
4. Embed the public half into `install.sh` (the `GROVE_TRUSTED_KEYS` heredoc). On rotation, add the new key to the trust-anchor array one release before it starts signing (overlap rule).
5. Store the private half as the `GROVE_MANIFEST_SIGNING_KEY` secret on the `release` environment (repository Settings -> Environments -> release -> Environment secrets). The secret value is the full PEM file content. Confirm presence without revealing it:
   ```bash
   gh api repos/alxshelepenok/grove/environments/release/secrets --jq '.secrets[].name'
   ```
6. Copy the private half to the offline backup medium; store it physically separated from the workstation.
7. Shred the staging copy after steps 5 and 6 are confirmed:
   ```bash
   rm -P grove-manifest-YYYY-MM.pem 2>/dev/null || { shred -u grove-manifest-YYYY-MM.pem 2>/dev/null || rm -f grove-manifest-YYYY-MM.pem; }
   ```
8. Record the execution (key id, fingerprint, date, where the backup lives) in the log below.

## Rollback

- Key not yet used for any release: delete the environment secret, remove the public key from `docs/security/artifacts/public-keys/` and from `install.sh`, destroy backups, generate a fresh keypair.
- Key already signed releases: follow `rotate-signing-key.md` instead (overlap rotation); destroying the only copy of the private half forces an emergency rotation, not a rollback.

## Execution log

| Date | Key id | Fingerprint (SPKI SHA-256 prefix) | Executed steps | Pending |
| --- | --- | --- | --- | --- |
| 2026-08-05 | `grove-manifest-2026-08` | `72d7aabe9c2ffcb4` | 1-8 | - |
