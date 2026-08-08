# Install

> Releases start with `v0.1.1`. Until the first release is published, the one-liner and the verify-then-run flow cannot work: `install.sh.sig`, `install.ps1.sig`, and `manifest.json` only appear once `v0.1.1` is out. Use [From source](#from-source-developers) in the meantime.

## Quickstart

Requires `bash`, `curl`, and `openssl` (on Windows use Git Bash, which ships all three).

```bash
curl -fsSL https://raw.githubusercontent.com/alxshelepenok/grove/main/install.sh | bash
```

Windows also has a native PowerShell installer with the same trust chain (RSA-PSS verified in-process via RSACng):

```powershell
iwr https://raw.githubusercontent.com/alxshelepenok/grove/main/install.ps1 -UseBasicParsing | iex
```

The installer downloads the signed release manifest, verifies its RSA-2048/PSS signature against the embedded release key *before parsing it*, then downloads the `grove` and `grove-mcp` binaries plus the `grove-desktop` app for your platform and checks their SHA-256 and size against the manifest *before installing* into `~/.local/grove` (`%USERPROFILE%\.local\grove` on Windows). Add `~/.local/grove/bin` to your `PATH`.

## Verify before you run

For the cautious, a two-step variant:

```bash
curl -fsSLO https://raw.githubusercontent.com/alxshelepenok/grove/main/install.sh
curl -fsSLO https://raw.githubusercontent.com/alxshelepenok/grove/main/install.sh.sig
```

Verify install.sh.sig against docs/security/artifacts/public-keys/grove-manifest-2026-08.pem, then:

```bash
bash install.sh
```

Useful options: `--only grove-mcp` (or `grove` / `grove-desktop`) installs a single component, `--version X.Y.Z` pins a specific release, `--self-test` exercises the verification logic against a local fixture. Updating uses the same command; the installer refuses manifests older than what is already installed (anti-rollback state in `~/.grove/.sequence`).

## Platform notes

- macOS: binaries are unsigned. Command-line downloads (`curl`) do not set the quarantine attribute, so the script path never touches Gatekeeper; if you download an artifact with a browser, clear it via `xattr -d com.apple.quarantine <file>`.
- Desktop OS bundles (`.msi`, `.dmg`, `.deb`, `.AppImage`) on the Releases page are unsigned: Windows SmartScreen and macOS Gatekeeper will warn on browser downloads. The signed `SHA256SUMS.sig` and `manifest.json.sig` attached to every release are the verification path (see the release notes for the exact commands).
- `grove-desktop` runtime prerequisites: Windows needs the WebView2 Runtime (preinstalled on Windows 11 and most Windows 10 installs); Linux needs `libwebkit2gtk-4.1` (`sudo apt install libwebkit2gtk-4.1-0` on Debian/Ubuntu); macOS works out of the box.
- Manual: archives, `SHA256SUMS`, the signed manifest, and the SBOM are attached to every [GitHub Release](https://github.com/alxshelepenok/grove/releases) if you prefer to verify and unpack by hand.

## From source (developers)

The Julia implementation of the CLI needs Julia 1.10+:

```bash
git clone https://github.com/alxshelepenok/grove.git ~/.local/grove
echo "alias grove='julia --project=$HOME/.local/grove/packages/grove $HOME/.local/grove/bin/grove.jl'" >> ~/.bashrc
```

## First session

```bash
grove init                  # inside your project: creates .grove/state.lock
grove next                  # highest-priority ready work on the critical path
grove packet W-01           # exactly the context this step needs, nothing more
grove check                 # verify all invariants (designed as a pre-commit hook)
grove fitness W-01 G-01 +1  # stage a metric delta against the goal
grove set W-01 status=done  # atomic close: deltas applied, goal re-derived, checksum written
```

## MCP server

`grove-mcp` exposes the full CLI as MCP tools over stdio (one tool per command, plus `grove://packet`, `grove://show/<id>`, and `grove://skill` resources - the last is a compact protocol primer, so an agent with only the MCP integration gets the behavioral minimum without the full skill). Build it from the repo root:

```bash
cargo build --release
```

The server takes `--root=<dir>` (the project whose `.grove/` lock it serves) and an optional `--session=<token>` (defaults to a per-process token).

Kimi Code CLI (`~/.kimi-code/mcp.json`):

```json
{
  "mcpServers": {
    "grove": {
      "transport": "stdio",
      "command": "/absolute/path/to/grove-mcp",
      "args": ["--root=/absolute/path/to/project"],
      "cwd": "/absolute/path/to/project"
    }
  }
}
```

Claude Desktop (`%APPDATA%\Claude\claude_desktop_config.json` on Windows, `~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
  "mcpServers": {
    "grove": {
      "command": "/absolute/path/to/grove-mcp",
      "args": ["--root=/absolute/path/to/project"]
    }
  }
}
```

Restart the client after editing the config; the `mcp__grove__*` tools appear in the next session.

## Agent skill bundle

The agent-facing workflow documentation (`docs/skills/`) ships as a single signed file, `grove-skill.md` (+ `grove-skill.md.sig`), attached to every release. Drop the file into your agent's skills directory as-is; verify it first with the committed public key (`docs/security/artifacts/public-keys/grove-manifest-2026-08.pem`) if you fetched it outside the installer.
