---
type: system
status: stable
---

# Bootstrap trust architecture

Describes how a fresh user establishes trust in Grove binaries: the installer is fetched over HTTPS, the release public key embedded in the installer is the trust anchor, and every subsequent byte is verified against a signed manifest before it is executed or installed.

## Problem

The documented install path is a pipe-to-shell one-liner (`curl ... | bash` or `iwr ... | iex`). The user runs code fetched from the network with no prior Grove material on the machine. The bootstrap must therefore carry its own trust anchor, verify everything it downloads before parsing or executing it, and degrade loudly rather than silently when verification is impossible.

## Trust anchor and distribution root

The trust anchor is the release public key embedded in the installer itself:

- `install.sh` holds the PEM in the `GROVE_TRUSTED_KEYS` heredoc.
- `install.ps1` holds the same key as `$TrustedModulusHex` plus exponent `65537` (consumed by `RSACng` without ASN.1 parsing).

The same public half is committed at `docs/security/artifacts/public-keys/grove-manifest-2026-08.pem`, so anyone can cross-check the embedded key against the repository. Both installers also ship detached signatures (`install.sh.sig`, `install.ps1.sig`) attached to every release, and `docs/install.md` documents a verify-before-run two-step variant for users who do not want to trust the first byte blindly.

The residual root of trust is GitHub plus TLS: the installer script itself is fetched from `raw.githubusercontent.com/alxshelepenok/grove/main/install.sh`, and an attacker who can swap that response swaps the embedded key with it. This is accepted deliberately for a single-maintainer OSS project (decisions D-05, D-09): the alternative (a separate distribution host, offline bootstrap signing, or a commercial code-signing certificate chain) adds operational cost without removing the need to trust some first byte. The committed public key and per-release installer signatures give researchers an out-of-band detection path.

## Verify-then-parse ordering

Both installers enforce a strict order, failing closed at every step:

1. Fetch `manifest.json` and `manifest.json.sig` (from the pinned release when `--version` / `-Version` is given, otherwise from the repository root on `main`).
2. Verify the RSA-2048/PSS signature over SHA-256 of the exact manifest bytes against the embedded key: `openssl dgst -sha256 -sigopt rsa_padding_mode:pss -sigopt rsa_pss_saltlen:-1 -verify` in `install.sh`, `RSACng.VerifyHash` in `install.ps1`. On failure: `manifest signature verification failed - refusing to parse or install`.
3. Only then extract fields. `install.sh` uses targeted `sed` extraction (never a JSON parser on unverified input); `install.ps1` runs `ConvertFrom-Json` only after verification.
4. Staleness: `expires_at` must not be more than 24 hours in the past (`now > expires_at + 86400` fails with `Manifest expired. A new release is pending; try again later.`).
5. Anti-rollback (below).
6. For each requested component, the artifact URL must start with `https://github.com/alxshelepenok/grove/releases/download/v<version>/` (host and tag pinning); size and SHA-256 of the downloaded archive must match the manifest before anything is unpacked or installed.

`install.sh --self-test` exercises this chain against a local fixture with an ephemeral key: happy path, tampered manifest, expired manifest, rolled-back sequence, and wrong-host URL are all asserted.

## Anti-rollback

State lives in `~/.grove/.sequence` (`%USERPROFILE%\.grove\.sequence`): an optional `format 1` header plus one `<channel>=<sequence>` line per channel, written atomically via temp file and rename.

- A manifest whose `sequence` is strictly lower than the stored value is rejected as a possible rollback. Equal sequence is accepted, so re-running the same install works.
- `MINIMUM_SEQUENCE=1` is embedded in both installers as a floor that survives state loss (new machine, deleted `.sequence`). On key compromise the floor is bumped so manifests signed before the compromise are refused even with no local state.
- The sequence is written only after every artifact verified and installed.

## Break-glass mode

`GROVE_ARTIFACT_URL=<url>` installs an archive directly, skipping manifest, signature, and hash verification. Both installers print loud warnings that trust is delegated to the user and the channel that delivered the URL, and neither touches the anti-rollback state. It exists as an explicit escape hatch (recovery, internal mirrors, forensics), never as a default.

Test hooks (`GROVE_FETCH_ROOT`, `GROVE_TRUSTED_KEY_FILE` / `GROVE_TRUSTED_MODULUS_HEX`, `GROVE_HOME`, `GROVE_SHORTCUT_ROOT`, `GROVE_USER_PATH_FILE`) similarly print a `trust/store override active` warning whenever set.

## Key rotation overlap rule

Installers currently embed a single key, so rotation is staged: the new public key is embedded in the installers (and committed under `docs/security/artifacts/public-keys/`) one release before it starts signing, and the workflow re-creates `install.sh.sig` / `install.ps1.sig` automatically when they stop verifying against the current key. Users who installed with any recent installer therefore already trust the new key when it takes over. See `key-management.md`.

## Install layout

Verification ends in a per-user install with no elevation:

- Prefix `~/.local/grove` (`%USERPROFILE%\.local\grove`): CLI binaries in `bin/`, the desktop app (binary plus `ui/views`, `ui/icons`) in `grove-desktop/`.
- PATH integration: an idempotent line in `~/.profile` (plus `~/.bashrc` / `~/.zshrc` when present); the per-user registry PATH on Windows.
- Launchers for the desktop app: `~/.local/share/applications/grove.desktop` (Linux), `~/Applications/Grove.app` (macOS stub), `Grove.lnk` on Desktop and in the Start Menu (Windows).

## Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Installer swapped in transit or on `main` | Per-release `install.sh.sig` / `install.ps1.sig`; committed public key for cross-check; verify-before-run variant in `docs/install.md`. |
| Release asset swapped on GitHub | SHA-256 and size in the signed manifest checked before install; URL host and tag pinning prevents redirect to attacker-controlled hosts. |
| Replay of an older still-signed manifest | `~/.grove/.sequence` anti-rollback plus `expires_at` staleness check; `MINIMUM_SEQUENCE` floor bounds the window after state loss. |
| Signing key compromise | Overlap rotation and `MINIMUM_SEQUENCE` bump; see `key-management.md`. |
| User needs an unverified install | `GROVE_ARTIFACT_URL` break-glass: explicit, loud, and never mutates trust state. |

## References

- `docs/architecture/release-distribution.md`: pipeline that produces and signs what the installers consume.
- `docs/architecture/key-management.md`: key inventory, storage, rotation.
- `install.sh`, `install.ps1`, `bin/verify.sh`.
- `docs/install.md`: user-facing instructions including the verify-before-run variant.
