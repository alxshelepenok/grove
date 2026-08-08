---
type: system
status: stable
---

# Key management architecture

Describes the cryptographic keys behind Grove's release trust system: what exists, where each half lives, who can use it, how rotation works, and which options were deliberately not taken.

## Problem

Every release artifact's integrity ultimately reduces to one signature: the signed `manifest.json` (and its companions `SHA256SUMS.sig`, `sbom.cdx.json.sig`, `vex.json.sig`, the installer signatures, and `grove-skill.md.sig`). The signing key must be usable by CI (releases are built and published by GitHub Actions), protected from casual exfiltration, recoverable if the CI secret is lost, and rotatable without breaking installed clients.

## Key inventory

Grove has exactly one long-lived signing identity. Key ids follow the pattern `grove-manifest-YYYY-MM`.

| Key | Algorithm | Purpose | Private half storage | Public half |
| --- | --- | --- | --- | --- |
| `grove-manifest-2026-08` | RSA-2048, PSS padding over SHA-256 | Signs `manifest.json`, `SHA256SUMS`, `sbom.cdx.json`, `vex.json`, `grove-skill.md`, and (on rotation) `install.sh.sig` / `install.ps1.sig`. | GitHub environment secret `GROVE_MANIFEST_SIGNING_KEY` on the `release` environment, plus an offline backup (encrypted USB drive or paper, stored physically separated from the workstation). | `docs/security/artifacts/public-keys/grove-manifest-2026-08.pem`; embedded in `install.sh` (`GROVE_TRUSTED_KEYS` heredoc) and `install.ps1` (`$TrustedModulusHex`, exponent 65537). |

The current key was generated on 2026-08-05; its SPKI SHA-256 fingerprint prefix is `72d7aabe9c2ffcb4` (execution log in `docs/security/runbooks/key-generation.md`).

There is no other long-lived key material in the system. Build provenance attestations use GitHub Actions OIDC (`id-token: write`), which is per-job ephemeral and GitHub-managed.

## Storage and access model

- The private half exists in exactly two places: the `GROVE_MANIFEST_SIGNING_KEY` environment secret and the offline backup. The generation runbook requires shredding the staging copy once both are confirmed (`docs/security/runbooks/key-generation.md`).
- The secret is scoped to the `release` GitHub environment, which is approval-gated: the `sign-and-publish` job cannot start, and therefore cannot see the key, until a maintainer approves the run. Dry-runs run in the separate `release-dry` environment and generate an ephemeral in-job key, so routine pipeline testing never touches the real secret.
- Inside the job the key is written with `umask 077` / `chmod 600` and used only via `bin/sign.sh`. As an integrity check the workflow prints the SPKI SHA-256 prefixes of both the secret-derived public key and the committed `trusted.pem`, making a mismatched secret visible in the log without revealing key material.
- The public half is public by design: committed in the repo, embedded in both installers, and printed in install output (`verify any artifact again with: bin/verify.sh ...`).

CI signing was a deliberate decision (D-05): an offline or hardware-backed signing ceremony was rejected for this single-maintainer OSS project because the operational cost exceeds the risk reduction, and the accepted mitigations are the approval gate on the `release` environment, the environment-scoped secret, dry-run isolation, and the offline backup that enables recovery and rotation if the CI secret is compromised.

## Rotation policy

- **Scheduled**: annually. A new `grove-manifest-YYYY-MM` keypair is generated per the key-generation runbook.
- **Overlap rule**: the new public key is committed under `docs/security/artifacts/public-keys/` and embedded in the installers one release before it starts signing (runbook step 4). The release workflow detects the change automatically: it re-creates `install.sh.sig` / `install.ps1.sig` only when the existing signatures no longer verify against the current key. Clients who installed from any recent installer therefore already trust the new key when it takes over; the old key is retired once installs pre-dating the overlap have aged out.
- **Incident rotation**: on suspected compromise, rotate immediately and bump `MINIMUM_SEQUENCE` in both installers, so manifests signed before the compromise are refused even on machines with no anti-rollback state (see `bootstrap-trust.md`). The overlap rule is abandoned in this case: the compromised key must stop signing at once.
- **Rollback vs rotation**: a key that has never signed a release can simply be deleted (secret removed, public key uncommitted, backups destroyed); a key that has signed releases can only be rotated forward, never rolled back (runbook rollback section). The rotation procedure is documented in `docs/security/runbooks/rotate-signing-key.md`.

## What is deliberately not used

- **Offline or hardware-backed signing** (YubiKey, TPM, air-gapped host): rejected by D-05 for cost and single-maintainer operability; mitigations listed above. The offline backup exists, but day-to-day signing is CI-only.
- **PGP / GnuPG signatures**: adds a second trust-anchor format and tooling dependency for no gain over the embedded-RSA verify path that both installers already implement (`openssl dgst` and `RSACng.VerifyHash`).
- **Cosign / Sigstore bundles**: installers accept only the plain base64url detached RSA-PSS signature; keyless infrastructure would add a runtime dependency on the Sigstore trust root.

The deferred options and the triggers that would reopen them (e.g. a second maintainer, commercial distribution, or a compliance regime requiring hardware-backed keys) are recorded in `docs/security/compliance.md`.

## Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| CI secret exfiltrated | Approval gate limits exposure to approved runs; secret never present in dry-runs; incident rotation with `MINIMUM_SEQUENCE` bump invalidates attacker-signed manifests. |
| CI secret lost (account or settings loss) | Offline backup restores the secret without breaking the installed base. |
| Maintainer machine compromised during generation | Runbook hygiene: staging outside the repo, `chmod 700` / `chmod 600`, shred after distribution; fingerprint log makes substitution detectable. |
| Rotation breaks existing installs | Overlap rule: new key embedded one release ahead; installer re-signing is automatic in the workflow. |
| Wrong secret uploaded to the environment | SPKI fingerprint prefix printed in the job log against the committed public key; mismatch is visible before anything ships (the smoke test would also fail). |

## References

- `docs/security/runbooks/key-generation.md`: generation ceremony and execution log.
- `docs/security/runbooks/rotate-signing-key.md`: rotation procedure.
- `docs/architecture/bootstrap-trust.md`: how installers consume the public key, `MINIMUM_SEQUENCE`, overlap rotation from the client side.
- `docs/architecture/release-distribution.md`: the `release` environment gate and what gets signed per release.
- `bin/sign.sh`, `bin/verify.sh`, `docs/security/artifacts/public-keys/`.
- `docs/security/compliance.md`: deferred options and revisit triggers.
