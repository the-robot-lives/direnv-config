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
dc load <file> [<file>...]  # ingest external YAML file(s): subject -> body, or a subjects: wrapper
                          # (--layer default for subjects without _dc_layer, plus the yaml flags)
dc get <name> [path]      # read a value (--raw)
dc set / unset / bump     # mutate values / version counter
dc list / status / env    # configs, merge state, resolved env
dc gen / gen-secrets      # generate values
dc encrypt / decrypt      # secret material
dc secrets                # list every in-scope key name (.envrc* heredocs + dc-loaded files + store layers)
                          # (--sources annotates each key with its file:line / store:<subject>/<layer> origins)
dc seal                   # interactive seal/unseal selector over project-scope sources (never touches store files)
                          # (--list inventory; --headless --encrypt/--decrypt KEY… [--dry-run] [--yes];
                          #  --match <PAT> / --except <PAT> (repeatable) select by LEAF KEY NAME only — the last
                          #  dot-segment (db_password of app.db.password), never the subject or full path;
                          #  patterns are case-insensitive wildcards, anchored to the whole name:
                          #  `*` = any run of characters (incl. none), `?` = exactly one char, all else literal;
                          #  --match patterns OR together, --except always wins over --match; selectors apply in
                          #  every mode (--list, TUI, headless, explicit or bulk actions) and compose with --dry-run
                          #  and --json (which reports `selected` + `selected_keys`); matching zero keys exits
                          #  non-zero — never a silent success;
                          #  decrypt restores `🔒 <value>` plaintext-at-rest for bare originals;
                          #  multi-line block-scalar values collapse to one quoted line on rewrite (known limitation);
                          #  ⛔ shadow keys are never modified)
dc emit                   # store -> loadable YAML (inverse of `dc load`): 🔒 tokens byte-identical, plain values plain
                          # (--output secrets.dc.yaml; layer variants gain .<layer>.yaml with _dc_layer routing;
                          #  _dc/auto get their own files + printed dc load --replace/--if-missing invocations;
                          #  --subject filters, --dry-run previews, targets must be .gitignore-covered unless --force)
dc infisical <sub>        # infisical get|set|compare — sync dc secrets against Infisical
dc compare <name> <path> --to <target>
dc push <name> <path> --to <target>    # force-push local secret(s) to a remote (--dry-run, --yes)
dc bat --all --flat --filter-key <regex>
dc config <sub>           # locate and edit values in source .envrc* and dc-loaded files
                          # --file <path> forces the destination (validated against known sources)
dc keys <sub>             # manage the lockable encryption-key file (status|migrate|lock|unlock)
dc prune / purge          # hygiene
dc init                   # scaffold a store
```

In `.envrc`: `dc_yaml` (deep merge, preserves sibling keys — the workhorse), `dc_yaml <name> --replace` (overwrite a whole named config's layer), `dc_load <file>...` (same ingest path for external YAML files; later files win on conflict), plus export helpers like `dc_export` that flatten per `_dc/` flatten rules. All functions are idempotent.

## How It Works

- Each named config lives in its own store directory with `base.yaml` (git-committable), env overlays, `local.yaml` (personal/IPC writes), `secrets.yaml` (gitignored), `.active` (resolved merge snapshot), and a `.version` counter.
- Config sources: `dc_yaml` heredocs in `.envrc*` plus any YAML files registered via `dc load` (tracked, with mtime/size, in the store's `.meta`). `dc config get/set` locate keys across both; edits splice the owning file and new keys land in the last-loaded file unless `--file` overrides.
- `dc config set -p/--parents` creates missing parent sections: `dc config set secrets inference.jev.api_key --value X -p --below sendgrid` inserts `inference:`/`jev:` as nested mappings and the leaf carries the 🔒 marker/token exactly like a plain `set`. Placement of the new top-most missing section among its siblings (mutually exclusive): `--below <PATH>` lands after the anchor's whole block, `--above <PATH>` before the anchor line, `--auto` takes the alphabetical spot (the default with `-p`; appends at the end with no siblings). Without `-p` a missing parent still fails — the error now hints at `-p` — and `--above/--below/--auto` without `-p` are rejected (they only matter when a section is actually missing; `-p` with every parent already present simply honours the placement for the new leaf).
- Every mutating call regenerates `.active` and writes a timestamped snapshot to `history/`.
- Flattening rules in `_dc/base.yaml` decide which keys become env vars (`cluster.kubeconfig` → `KUBECONFIG`, etc.).
- Integrations in `src/`: Infisical sync, Kubernetes (kubeconfig handling), crypto/secret tooling, audit and shadowing.
- `bin/dc-init` scaffolds projects; `bin/tabbing-on-step` feeds terminal tab-title state.

## Docs

Extensive documentation in `docs/` — `PROJ-ARCH`, `PROJ-LAYOUT`, `PROJ-SCHEMA`, `PROJ-FAQ`, `PROJ-HOWTO`, `THREAT-MODEL`, `TEST-HEALTH` (each with a `.summary.md` digest), task howtos in `docs/howto/`, internals in `docs/arch/` (flatten rules, layer resolution, parent chain) and `docs/layout/`; `demo/` has a runnable walkthrough.
