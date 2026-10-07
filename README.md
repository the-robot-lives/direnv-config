# direnv-config

**Repo:** https://github.com/the-robot-lives/direnv-config

YAML-backed configuration layer for [direnv](https://direnv.net/) — replaces sprawling `export VAR=value` `.envrc` files with structured, versioned, mergeable YAML configs, and exposes them as env vars automatically. MIT licensed.

## What

The `dc` CLI (Rust) plus a direnv shell library. Projects declare named configs (`cluster`, `cloudflare`, `tab`, …) in a one-liner `.envrc`; values are stored as layered YAML (`base.yaml` → env overlay → `local.yaml` → `secrets.yaml`) under `~/.local/state/direnv-config/{path-hash}/`, deep-merged, and flattened into shell env vars by `dc_export`.

Key insight: env vars are write-once from a parent process's perspective — but a child can edit a file. Because state lives in files, any process (parent, child, sibling, Claude Code) can read or mutate shared config; every write bumps a monotonic version counter and snapshots to `history/`.

## Why

| Problem | direnv-config fix |
|---|---|
| `.envrc` files grow 50+ export lines | One-liner `.envrc` loading structured YAML |
| No hierarchy or grouping | Named configs with nested keys |
| Child processes can't update parent config | Edit the YAML; parent reads next access |
| No layering (dev vs prod vs local) | Elixir-style deep merge across layer files |
| Secrets mixed with plain config | Separate `secrets.yaml` (gitignored) |
| No history | Timestamped snapshots on every write |
| No sharing with external state (tab titles, etc.) | `dc_set tab status "deploying"` — file-based IPC |

## Getting Started

Prerequisites: `direnv` hooked into your shell; Rust toolchain to build.

```bash
make compile          # cargo build
make test
make install          # binary + direnv lib + shell hook
make install-completions
make doctor           # environment check
make sdk-build        # / sdk-test / sdk-publish — multi-language SDK clients
```

Then in a project `.envrc`:

```bash
dc_yaml cluster <<'YAML'
name: noizu
kubeconfig: ~/.kube/noizu/config
YAML
dc_export
```

## The `dc` CLI

```bash
dc yaml <name>            # merge YAML from stdin (--replace, --replace-key, --if-missing, --layer)
dc get <name> [path]      # read a value (--raw)
dc set / unset / bump     # mutate values / version counter
dc list / status / env    # configs, merge state, resolved env
dc gen / gen-secrets      # generate values
dc encrypt / decrypt      # secret material
dc secrets / secretsmap / secretcmp
dc infisical <sub>        # infisical get|set|compare — sync dc secrets against Infisical
dc compare <name> <path> --to <target>
dc bat --all --flat --filter-key <regex>
dc config / settings      # tool configuration
dc audit / prune / purge  # hygiene
dc init                   # scaffold a store
```

In `.envrc`: `dc_yaml` (deep merge, preserves sibling keys — the workhorse), `dc_yaml <name> --replace` (overwrite a whole named config's layer), plus export helpers like `dc_export` that flatten per `_dc/` flatten rules. All functions are idempotent.

## How It Works

- Each named config lives in its own store directory with `base.yaml` (git-committable), env overlays, `local.yaml` (personal/IPC writes), `secrets.yaml` (gitignored), `.active` (resolved merge snapshot), and a `.version` counter.
- Every mutating call regenerates `.active` and writes a timestamped snapshot to `history/`.
- Flattening rules in `_dc/base.yaml` decide which keys become env vars (`cluster.kubeconfig` → `KUBECONFIG`, etc.).
- Integrations in `src/`: Infisical sync, Kubernetes (kubeconfig handling), crypto/secret tooling, audit and shadowing.
- `bin/dc-init` scaffolds projects; `bin/tabbing-on-step` feeds terminal tab-title state.

## Docs

Extensive documentation in `docs/` and the Sphinx/ReadTheDocs setup; `demo/` has a runnable walkthrough.
