---
type: runbook
status: stable
---

# Rotating the release signing key

Replace the release signing keypair, either on the annual schedule or after a suspected compromise. Grove has no offline signing host: signing happens in CI inside the approval-gated `release` environment, so rotation means swapping the `GROVE_MANIFEST_SIGNING_KEY` environment secret and the trust anchors embedded in the installers. Rotation keeps an overlap window so installers downloaded before the switch still verify.

## Prerequisites

- Completed `key-generation.md` at least once (staging directory, offline backup medium, fingerprint record).
- Write access to repository settings (environments and secrets) and approver rights on `release`.
- For incident rotation: `incident-response.md` is already activated.

## Steps

1. Generate the new keypair per `key-generation.md` steps 1-2, named `grove-manifest-YYYY-MM` (year-month of generation). Record the fingerprint.
2. Publish the public half:
   ```bash
   cp grove-manifest-YYYY-MM.pub.pem docs/security/artifacts/public-keys/grove-manifest-YYYY-MM.pem
   ```
3. Add the new trust anchor to the installers ONE RELEASE BEFORE the new key starts signing (overlap rule):
   - `install.sh`: append the new PEM block to the `GROVE_TRUSTED_KEYS` heredoc; the installer trusts every key in the heredoc.
   - `install.ps1`: it embeds a single RSA modulus (`$TrustedModulusHex`), not a list, so it cannot hold two keys. Derive the new modulus hex for the later swap and keep it with the change:
     ```bash
     openssl rsa -in grove-manifest-YYYY-MM.pem -modulus -noout | cut -d= -f2
     ```
     The ps1 modulus is switched only in the release where the new key starts signing (step 6).
4. Ship a normal release (per `publish-release.md`) with the old key still signing. From this release on, fresh `install.sh` downloads trust both keys.
5. Swap the environment secret on the `release` environment (Settings -> Environments -> release -> Environment secrets), or:
   ```bash
   gh secret set GROVE_MANIFEST_SIGNING_KEY --env release < grove-manifest-YYYY-MM.pem
   ```
   Confirm presence without revealing it:
   ```bash
   gh api repos/alxshelepenok/grove/environments/release/secrets --jq '.secrets[].name'
   ```
6. Point the pipeline at the new key, in the same commit:
   - `.github/workflows/release.yml`: update the "Prepare signing key" step (`cp docs/security/artifacts/public-keys/grove-manifest-2026-08.pem trusted.pem`) to the new public key file.
   - `install.ps1`: replace `$TrustedModulusHex` with the new modulus hex from step 3.
   - `install.sh`: update the pubkey path in the final verification hint printed by the installer.
7. Ship the next release. It is signed by the new key; `install.sh` (both anchors) and fresh `install.ps1` (new modulus) verify it.
8. Copy the new private half to the offline backup medium, stored physically separated from the workstation.
9. Shred the staging copy after steps 5 and 8 are confirmed:
   ```bash
   rm -P grove-manifest-YYYY-MM.pem 2>/dev/null || { shred -u grove-manifest-YYYY-MM.pem 2>/dev/null || rm -f grove-manifest-YYYY-MM.pem; }
   ```
10. After one full release cycle on the new key, remove the retired PEM from the `install.sh` heredoc and delete the old public key file only when no supported manifest is still signed by it.
11. Record the rotation in the execution log of `key-generation.md` (key id, fingerprint, dates of each step).

## Suspected compromise additions

When rotation is triggered by compromise rather than schedule:

- Activate `incident-response.md` first; its 1-hour actions (revoking the secret) come before key generation.
- Bump the anti-rollback floor so manifests signed by the old key are rejected even if an attacker held a valid older manifest: set `MINIMUM_SEQUENCE` in `install.sh` and `$MinimumSequence` in `install.ps1` to the sequence of the first manifest signed by the new key. This ships with the release in step 7.
- Skip the relaxed overlap timeline: perform steps 5-7 in one emergency release instead of waiting a full cycle.

## Rollback

- New key not yet used for any release: delete the environment secret (restore the old value from the offline backup), revert the installer/workflow edits, destroy the new key per `key-generation.md` rollback.
- New key already signed a release: there is no rollback; rotate forward with a third key following this same procedure.
