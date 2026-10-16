# direnv-config

**Repo:** https://github.com/the-robot-lives/direnv-config · MIT licensed

`direnv-config` is a YAML-backed configuration layer for [direnv](https://direnv.net/). The `dc` CLI (Rust) plus a direnv shell library replace sprawling `export VAR=value` `.envrc` files with structured, versioned, mergeable YAML configs that flatten into env vars automatically. Key insight: env vars are write-once from a parent process's perspective — but state lives in files under `~/.local/state/direnv-config/{path-hash}/`, so any process (parent, child, sibling, Claude Code) can read or mutate shared config; every write bumps a monotonic version counter and holds an advisory `.lock` so concurrent shells can't clobber each other.

## Install

Prerequisites: [direnv](https://direnv.net/) hooked into your shell; a Rust toolchain to build.

```bash
make compile              # cargo build --release
make test
make install              # dc binary + dc-init + tabbing-on-step, direnv lib, shell hook, completions
make install-completions  # bash + zsh completions (also part of install)
make doctor               # diagnose install issues (make check = quick verify)
```

## Quick start

Declare a named config in your `.envrc` (convention: keep them in a separate `.envrc.dc` — `dc` scans every `.envrc*` in the project):

```bash
dc_yaml cluster <<'YAML'
name: noizu
kubeconfig: ~/.kube/noizu/config
YAML
dc_export
```

After `direnv allow`: `dc get cluster name` reads a value, `dc env` shows the flattened exports (what `dc_export` evals), `dc status` shows merge state.

## Core concepts

- **Named configs.** Each name (`cluster`, `cloudflare`, `tab`, …) is a store directory of YAML layer files, deep-merged in a fixed order: `base.yaml` (git-committable) → `$DC_ENV` overlay → `local.yaml` (personal/IPC writes) → `secrets.yaml` (gitignored). Layer weights are configurable in `base.yaml` (`configuration.dc.layers`).
- **Flattening.** Rules in the `_dc` config decide which keys become env vars (`cluster.kubeconfig` → `KUBECONFIG`); `dc_export` evals `dc env`.
- **Value resolution** (`dc get`): `--override ENV` (env wins) → stored dc value → `--fallback ENV` (dc wins) → `--auto` generated → `--default`. `--auto` generates on miss but does **not** persist the value into your config (a first-draw memo lands in `secrets/.envrc.auto`, which nothing sources) — use `dc config set` for durable values.
- **Sealed secrets.** `dc encrypt` wraps plaintext as `🔒:v1:t<tier>:…` tokens stored at rest; `dc decrypt` / `dc get --reveal` / `dc bat --reveal` read them back (audited). ⛔ marks shadowed keys moved out of band (`dc config secure`).
- **Shell functions.** `dc_yaml`, `dc_load`, `dc_get`, `dc_set`, `dc_unset`, `dc_prune`, `dc_bump`, `dc_export` — thin, idempotent wrappers over `dc` for use in `.envrc`.

## Command reference

Run `dc --help` (or `dc <command> --help`) for full flags.

```bash
dc yaml <name>                     # merge YAML from stdin (--replace --replace-key --if-missing --layer)
dc load <file>...                  # ingest external YAML files; root keys = subjects (later files win)
dc get <name> [path]               # read a value (--raw --override --fallback --auto --default --reveal --clippy)
dc set / unset / prune / bump      # mutate values / remove keys or branches / bump .version
dc env                             # resolved env vars (--list --diff); what dc_export evals
dc list / status                   # known stores / current merge state
dc gen / gen-secrets               # random secrets (encrypted by default) / fill 🎲 placeholders
dc encrypt / decrypt               # 🔒:v1 token round-trip
dc secrets                         # every in-scope key name, never values (--json --sources)
dc emit                            # store -> loadable YAML (inverse of dc load; gitignore-gated)
dc bat [subject]                   # browse/search with secrets masked (--all --flat --filter-key --reveal)
dc compare <name> <path> --to <target>   # compare a local secret against remote targets (no value output)
dc push <name> <path> --to <target>      # force-push secrets (infisical:// kubernetes://) (--dry-run --yes)
dc infisical get|set|compare       # round-trip dc secrets against Infisical
dc config get|set|setall|secure    # locate/edit values in source .envrc* files
dc keys status|migrate|lock|unlock # manage the external encryption-key file
dc init / purge                    # scaffold a store / hard-delete a config or store
dc seal                            # encryption inventory + in-place (un)seal
```

`dc config set <subject> <path> --value X [--yes]` splices the owning `.envrc*`/registered file; `-p/--parents` builds missing parent sections (`--below/--above/--auto` place the new top-most section; alphabetical is the default with `-p`). Missing parents without `-p` fail with a hint.

`dc seal` — `--list [--json]` prints `key | file:line | state`; interactive selector on a tty, `--headless --encrypt/--decrypt KEY...` (or `--encrypt-all-plain`/`--decrypt-all`) otherwise. Repeatable `--match <PAT>` / `--except <PAT>` select by leaf key name only (case-insensitive wildcards: `*` any run of chars, `?` exactly one; `--except` wins; zero matches exits non-zero). `--file`, `--dry-run`, `--yes` compose in every mode. Store layer files are never rewritten and ⛔ shadow keys are never modified. Known limitation: multi-line block scalars collapse to one quoted line on rewrite.

## SDKs

Client libraries in Rust, TypeScript, Python, Elixir, and PHP share one API surface (`get_string`, `set`, …) for reading and writing dc stores from application code. `make sdk-build` / `sdk-test` / `sdk-publish` fan out to all five. See [sdk/README.md](sdk/README.md).

## Docs

- Architecture: [docs/PROJ-ARCH.md](docs/PROJ-ARCH.md) (digest: [PROJ-ARCH.summary.md](docs/PROJ-ARCH.summary.md)); internals in [docs/arch/](docs/arch/) — flatten rules, layer resolution, parent chain.
- Repo layout: [docs/PROJ-LAYOUT.md](docs/PROJ-LAYOUT.md) (digest: [PROJ-LAYOUT.summary.md](docs/PROJ-LAYOUT.summary.md)); per-directory notes in [docs/layout/](docs/layout/).
- More: schema ([docs/PROJ-SCHEMA.md](docs/PROJ-SCHEMA.md)), FAQ ([docs/PROJ-FAQ.md](docs/PROJ-FAQ.md)), howtos ([docs/PROJ-HOWTO.md](docs/PROJ-HOWTO.md), [docs/howto/](docs/howto/)), threat model ([docs/THREAT-MODEL.md](docs/THREAT-MODEL.md)), test health ([docs/TEST-HEALTH.md](docs/TEST-HEALTH.md)) — each has a `.summary.md` digest except TEST-HEALTH.
- `demo/` has a runnable walkthrough; `web/` is the Elixir/Hologram landing site, deployed via `helm/dirconfig-site`.
