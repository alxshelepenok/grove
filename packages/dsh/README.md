# dsh-grove

A [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) (dsh) bundle that exposes the [Grove](https://github.com/alxshelepenok/grove) CLI as agent tools.

The plugin is a thin wrapper: every tool builds argv, execs the `grove` binary, and returns raw stdout. All protocol invariants (DoR gates, evidence gates, WIP limit, lockfile checksum) stay enforced by the Grove CLI itself; nothing is reimplemented here.

## Prerequisites

- Node.js 20+.
- The `grove` binary on `PATH` (or an absolute path via the `bin` config key).
- A dsh installation: `npx @deepseek-ai/dsh --help`.

## Install into a profile

```sh
dsh plugin --profile <name> add /path/to/grove/packages/dsh-plugin
dsh --profile <name> --dump-config
```

The package declares `dsh.bundle.patch`, so `dsh plugin add` appends the bundle to the profile automatically. The shipped `cordis.patch.yml` mounts one row:

```yaml
- insert:
    - id: grove
      name: dsh-grove
      config:
        bin: grove
```

Override `bin` in the profile's own `cordis.patch.yml` (later layers win per row; restate every key):

```yaml
- insert:
    - id: grove
      name: dsh-grove
      config:
        bin: /opt/grove/bin/grove
```

## Tools

Read-only (safe to call any time):

| Tool | Grove command |
| --- | --- |
| `grove_status` | `grove status` |
| `grove_next` | `grove next` |
| `grove_ready` | `grove ready` |
| `grove_path` | `grove path` |
| `grove_packet` | `grove packet <id> [--cone --cone-depth=N --cone-max=N]` |
| `grove_show` | `grove show <id>` |
| `grove_list` | `grove list <kind> [--status= --cynefin=]` |

Mutating (guarded by CLI invariants; a refusal such as `DoR ≢ ⊤` surfaces as the tool error):

| Tool | Grove command |
| --- | --- |
| `grove_add` | `grove add <kind> --title=... [--type= --cynefin= --area= --goals= --theme= --fitness-kind= --fitness-target=]` |
| `grove_field` | `grove field <id> <field> add\|rm\|clear [value]` |
| `grove_set` | `grove set <id> <key>=<value>` |
| `grove_evidence` | `grove evidence <id> <text>` |
| `grove_fitness` | `grove fitness <id> <goal> <±delta>` |
| `grove_link` | `grove link <from> <label> <to>` |

## Session ownership

Grove claims work items per session (I11). The plugin forwards the ambient `GROVE_SESSION` environment variable to the CLI automatically; every mutating tool also accepts an optional `session` parameter that is forwarded as `--session=<token>`.

## Development

```sh
bun install
bun run build
bun run typecheck
bun test
```

`GROVE_PROJECT=<scratch-dir> node scripts/round-trip.mjs` drives the built tools through a full `add -> field -> fitness -> evidence -> done` cycle against a scratch Grove project.
