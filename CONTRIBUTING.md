# Contributing

Grove is a single-maintainer open-source project (AGPL-3.0). Contributions are welcome; keep them small, evidenced, and in line with the rules below. By contributing you agree your changes are licensed under the project's license.

## Running tests

Rust workspace (CLI, MCP server; excludes the desktop app):

```bash
cargo test --workspace --exclude grove-desktop --locked
```

Installer suites:

```bash
bash tests/install/test-install.sh
powershell tests/install/test-install.ps1
```

Julia reference implementation: `make test`.

A change is done when the suites covering it pass. If you touch the release or audit pipeline, also run the relevant suites under `tests/` (`tests/audit`, `tests/manifest`, `tests/vex`, `tests/crypto`).

## Code style

- English everywhere: identifiers, docs, commit messages.
- No code comments. Code is self-documenting: name things precisely, keep functions small, and let tests carry the intent. Doc comments are acceptable only where a public contract is genuinely non-obvious.
- ASCII hyphens in prose and docs; no en/em dashes.
- Match the surrounding file's conventions rather than importing your own.

## Dependency policy

The dependency graph is attack surface and audit surface; the signed SBOM (`docs/security/artifacts/sbom.cdx.json`) covers every release, so adding a dependency is a security-relevant change. A new dependency must satisfy all of the following:

1. **Necessary.** No standard library facility and no existing dependency can do the job without disproportionate effort. "It saves twenty lines" is not necessity.
2. **Permissively licensed.** MIT, Apache-2.0, BSD (2/3-clause), or ISC. Copyleft (GPL-family, MPL, ...) in a dependency needs explicit discussion in the PR before merge, even though Grove itself is AGPL.
3. **No install-time execution or network access.** No install scripts and no build-time network fetches: `build.rs` doing network access is a red flag, as are heavy JLL packages on the Julia side. Dependencies that phone home at runtime are disqualified outright.
4. **Audited.** The dependency must pass the weekly trivy pipeline (`bin/audit.sh` over `Cargo.lock` and `packages/grove/Manifest.toml`, fail-closed on schedule and at release). PR checks run the same gate fail-open; a new CRITICAL/HIGH finding on your dependency blocks the next release.

Propose new dependencies in the PR description: what it does, why the alternatives fail, its license, and its build-time behavior.

## Pull requests

1. Fork and branch from `main`. Keep the diff scoped to one concern.
2. Run the test suites above locally before opening the PR.
3. Describe the change and its motivation in the PR body; link the issue if one exists.
4. CI must be green (rust tests, shell tests, security scan). The maintainer reviews and merges; direct pushes to `main` are not possible.
5. Do not bump versions, create tags, or edit release artifacts (`manifest.json`, `docs/security/artifacts/`) in a PR - those are produced by the release workflow.

Security issues are not PRs: report them privately per `SECURITY.md`.
