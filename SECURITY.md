# Security policy

This is the public security policy for Grove. It covers the `grove` CLI, the `grove-mcp` MCP server, the `grove-desktop` application, and the release distribution pipeline (`install.sh`, `install.ps1`, signed manifests, release artifacts).

## Reporting a vulnerability

- **Primary channel:** [GitHub private vulnerability reporting](https://github.com/alxshelepenok/grove/security/advisories/new) for this repository. This keeps the report private until a fix is released and lets us request a CVE for HIGH/CRITICAL issues.
- **Fallback:** if private reporting is unavailable to you, open a regular GitHub issue containing *no* vulnerability details and ask for a private contact channel.

Please do not disclose vulnerability details in public issues, pull requests, or discussions before coordinated disclosure.

### What to include

A useful report contains:

- Affected version (e.g. `0.1.0`) and how it was installed.
- Affected component (CLI, MCP server, install script, release/signing pipeline, dependencies).
- Reproduction steps or proof-of-concept.
- Impact assessment.
- Suggested remediation, if any.
- Whether you intend to publish; preferred coordinated disclosure timeline.

### Response targets

Grove is maintained by a single developer. These are targets, not contractual SLAs:

| Stage | Target |
| --- | --- |
| Acknowledge receipt | 72 hours |
| Triage and severity assessment | 7 days |
| Fix for CRITICAL or HIGH | 7 days from triage, best-effort |
| Fix for MEDIUM | Next scheduled release |
| Fix for LOW | Best-effort, included in a future release |
| Coordinated disclosure window | 90 days from initial report |

### Safe harbour

We will not pursue legal action against researchers acting in good faith who:

- Do not access user data beyond what is necessary to demonstrate the vulnerability.
- Do not modify or destroy data.
- Do not disrupt service for other users.
- Disclose responsibly per the timeline above.

## Supported versions

Grove is pre-1.0 and ships a rolling release line:

| Status | Definition | Support |
| --- | --- | --- |
| Current | Latest release | Full security and feature support |
| Older | Any earlier release | No fixes; update to the latest release |

Security fixes ship as patch releases on the current line only; there are no backport branches. From 1.0 onward this policy extends to the latest plus the previous minor release.

## Vulnerability handling process

1. **Receipt and acknowledgement** within the target above.
2. **Triage**: severity (CRITICAL / HIGH / MEDIUM / LOW) based on CVSS 3.1 and exploit status.
3. **Fix development**: includes root cause analysis and a regression test. Findings in third-party dependencies are triaged into signed VEX statements (`docs/security/artifacts/vex.json`).
4. **CVE assignment**: for HIGH and CRITICAL, requested via GitHub Security Advisory.
5. **Release**: fix ships in a signed release; all artifacts are covered by the signed manifest.
6. **Public disclosure**: GitHub Security Advisory plus release notes, after the coordinated disclosure window or earlier if mutually agreed.
7. **Reporter acknowledgement**: credited in the advisory (with consent).

## Verifying releases

Before installing, you can verify any release artifact against the signed manifest; see `docs/install.md` (verify-then-run variant) for the trust model. Release signatures are produced by the dedicated Grove release key; the public half is committed under `docs/security/artifacts/public-keys/`.

## Out of scope

The following are not considered vulnerabilities for the purposes of this policy:

- Defects in the user's operating system, hardware, or shell environment.
- Findings in third-party dependencies that have been declared `not_affected` in the signed `vex.json` (see `docs/security/artifacts/`).
- Findings in test infrastructure, fixtures, or development tooling not shipped to users.
- Attacks requiring the ability to run arbitrary code as the user already (Grove is a local CLI; it does not defend the machine from itself).

Reports on these topics are still welcome as documentation or hardening input, but they do not trigger the response targets above.
