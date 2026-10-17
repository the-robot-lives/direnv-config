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

## Load YAML files (`dc load`)

A *load YAML file* is a plain YAML file that feeds a store the same way `dc_yaml` heredocs do — one file can declare many named configs at once, lives outside `.envrc`, and is diffable/committable like any YAML. `dc load <file>...` ingests it through the exact `dc yaml` pipeline (same split-secrets + encryption), registers it in the store's `.meta`, and bumps the version once at the end.

**Discovery is explicit.** dc does **not** search for load files — you name them on the command line (paths are canonicalized; the absolute path is recorded in `.meta`). What *is* discovered automatically:

| Source | How dc finds it |
|---|---|
| `.envrc*` heredocs | every `.envrc` and `.envrc.*` in the project directory (non-recursive, sorted) |
| load files | only where invoked: `dc load <file>...` (or `dc_load` in `.envrc`); re-loaded files re-register, in argument order |
| store for a directory | CWD, then each parent directory (`dc init` creates it) |
| parent stores | ancestor store dirs under `~/.local/state/direnv-config/`, merged oldest-first |
| `.infisical-secrets.yaml` | `$INFISICAL_SECRETS_FILE`, else an upward walk from CWD (drives `dc infisical` only) |

### File schema (annotated example)

Root mapping = subject name → config body. A lone `subjects:` wrapper key is an optional alternative; mixing it with other root keys is an error (refused, never guessed).

```yaml
# config/dc.yaml — committable, non-secret defaults (loaded first)
cluster:
  name: noizu
  region: us-east-1
  kubeconfig: ~/.kube/noizu/config

app:
  log_level: info
  db:
    host: localhost
    port: 5432

_dc:                          # special subject: flatten rules + export list
  flatten:
    cluster.kubeconfig: KUBECONFIG   # explicit rule: config path → env var
    app.db.*: APP_DB_*               # wildcard rule: app.db.host → APP_DB_HOST
  export:
    - app.log_level           # unmapped keys export as APP_LOG_LEVEL
```

```yaml
# secrets/dc.local.yaml — NEVER commit (gitignore it before creating);
# loaded second, so its keys win on conflict
app:
  db:
    password: "🔒❗ example-not-a-secret"      # 🔒 = secret → encrypted into secrets.yaml
  api_token: "🎲 api_tok 🔒 hex 32"            # 🎲 placeholder → dc gen-secrets fills in place
  report_key: "🔒❗❗❗ example-not-a-secret"   # ❗×1–3 = sensitivity tier (low → high)

cluster:
  _dc_layer: local            # reserved routing key: store this subject in the `local` layer
  context: minikube           # (stripped at ingest; never becomes a value)

# alternative shape — subjects: wrapper (same file, one style only)
# subjects:
#   cluster:
#     name: noizu
```

Value markers (identical to heredocs):

| Marker | Meaning |
|---|---|
| *(none)* | plaintext, stored into the subject's chosen layer |
| `🔒` | secret — encrypted to a `🔒:v1` token, **always** routed to the subject's `secrets` layer regardless of `--layer`/`_dc_layer` |
| `❗`×0–3 | sensitivity tier suffix on `🔒` |
| `🎲 <name> 🔒 <charset> <len>` | generator placeholder for `dc gen-secrets` |
| `⛔` | restricted sentinel — value lives in the shadow store only (`dc config secure`) |

A scalar that already *is* a `🔒:v1:…` token (padlock, no space) is stored verbatim — pasting at-rest tokens never double-wraps them. A subject whose body is entirely secrets gets no base-layer file at all.

### Precedence — who wins

```text
within one layer   ingest order decides: each load deep-merges over what's
                   already stored — later files/args win; there is no
                   "heredoc beats file" rule, only who loaded last
across layers      base(1000) → $DC_ENV(750, default dev) → local(500) → secrets(250)
                   smaller weight = merged later = wins; override per subject via
                   `configuration.dc.layers` (layer → weight) in its base.yaml
across stores      parent chain merges ancestor-first; a child's root-level
                   `_dc_pruned: true` breaks inheritance for that config
at read time       dc get: --override ENV → stored value → --fallback ENV
                   → --auto password|hex [len] → --default (miss = exit 1)
```

Deep-merge semantics: maps recurse; sequences and scalars **replace** wholesale (no element-wise merge); `_dc_pruned: true` tombstones prune the subtree.

### Subjects — what `dc get <subject>` means

`dc get`'s first positional is a **subject**: one named config, i.e. one subdirectory of the active store. `dc get services design.authentik_api_token` = subject `services`, YAML path `design.authentik_api_token`. The set is fully dynamic — no enum: a subject is any subdirectory of the store (`src/cmd/emit.rs:276` skips dot-dirs and `history`), created by `dc load` (root keys or the `subjects:` wrapper, `src/cmd/load.rs:38-58`), `dc config set`, or `dc init`. Reading resolves `<store>/<subject>/.active` deep-merged across the parent-store chain (`src/store/resolve.rs:57-87`; weights in [Precedence](#precedence--who-wins)). A value of the form `🔗 subject.path` is chased to its target, max depth 8 (`src/cmd/get.rs:6,29-47`).

Known names — code-reserved vs pure convention:

| Subject | What it's for | Status |
|---|---|---|
| `_dc` | flatten/export rules driving `dc env` (`src/cmd/env.rs:10`); emitted alone with `--replace` | reserved (`src/cmd/emit.rs:48`) |
| `auto` | conventional home for generated/memo values; emitted alone with `--if-missing`. Not magic to `dc get` — generation is the `--auto password\|hex [len]` flag (`src/cmd/get.rs:236-247`); the memo lands in `<project>/secrets/.envrc.auto` only when `--override`/`--fallback` named an env var (`src/cmd/get.rs:174-195`) | semi-special (`src/cmd/emit.rs:48`) |
| `secrets` | general/shared credentials (e.g. `secrets smtp.host`, `secrets google.marketing.*`), including Infisical's connection settings (`dc infisical` falls back to e.g. `secrets infisical.host`; `src/infisical.rs:50-68`) | convention |
| `services` | per-service credentials, referenced by `.infisical-secrets.yaml` `dc:` specs like `services.design.<key>` (`src/secretsmap.rs:61`) | convention |
| `apps` | per-app config/creds | convention only — nothing in code references it |
| *anything else* | whatever you load | free |

Subjects live at `~/.local/state/direnv-config/<path-hash>/<subject>/{base.yaml,<DC_ENV>.yaml,local.yaml,secrets.yaml,.active}` (`src/store/layout.rs:7-38`; layer order in [Precedence](#precedence--who-wins)).

Nothing maps a subject+key to an env var by itself — the only bridges are the `_dc` flatten/export rules ([Pitfalls](#pitfalls)), the shell lib (`dc_load`/`dc_yaml`/`dc_export`), and `.infisical-secrets.yaml` `dc:` specs. Reads are masked: `🔒` values print redacted unless `--reveal`/`--clippy`, `⛔` restricted needs `--reveal-restricted` (`src/cmd/get.rs:104-146`). Safe listing: `dc status` (subject names only, from `.meta`, `src/cmd/status.rs:16-19`), `dc list` (stores + config counts), `dc secrets` (key names per subject/layer, never values); `dc emit` re-renders structure with tokens, never values.

### Bootstrap a new project

1. Install once per machine: `make install` (binary, direnv stdlib copy at `~/.config/direnv/lib/dc.sh` — direnv sources that directory automatically, so `dc_load`/`dc_yaml`/`dc_export` work in any `.envrc` without imports — and the `dc-init` shell hook). `make doctor` verifies all three.
2. `cd <project> && dc init` — creates the store under `~/.local/state/direnv-config/<path-hash>/` (optional: `dc init --from-envrc .envrc` imports existing `export` lines into an `imported` config).
3. Write the load files: a committable `config/dc.yaml` for defaults, a `secrets/dc.local.yaml` for real values — **add the secrets file to `.gitignore` before creating it**: `🔒`-marked values sit as plaintext in the file until ingested; only the store holds ciphertext.
4. Wire direnv — keep the dc calls in `.envrc.dc` and source it from `.envrc`:
   ```bash
   # .envrc
   source_env .envrc.dc

   # .envrc.dc
   dc_load config/dc.yaml
   dc_load secrets/dc.local.yaml
   dc_export                    # evals `dc env`: DC_ROOT/DC_VERSION/DC_ENV + flatten rules
   ```
5. `direnv allow`, then verify: `dc status` (merge state), `dc env --list` (what would export), `dc get app db.host`.

### Everyday workflows

**Add a secret.** Prefer the source-splice command — it encrypts, edits the owning file in place (registered load files included), preserves comments, and re-ingests so the store updates immediately:

```bash
dc config get app db.password                       # locate: file:line + redacted preview
dc config set app db.password --value 'example-not-a-secret' --yes
```

Brand-new keys insert into the **last-loaded** file (registered externals first, then `.envrc*` heredocs); use `-p/--parents` to create missing parent sections (`--below/--above/--auto` place the new top-most section; alphabetical is the default), or `--file <path>` to pin the destination. A key found in two sources is refused with every candidate listed — pick one with `--file`.

**Store write vs source write.** `dc set app feature_flag true` writes the **store** (default layer `local`) and never touches your YAML; `dc config set` edits the **source file** and refreshes the store. Store-local values shadow source values on every resolution — after a `dc set`, editing the load file won't change what `dc get` returns until you `dc unset app feature_flag`. `dc config set` encrypts by default: `--plain` stores a literal value with no 🔒 marker (non-secret values — overwriting an encrypted value also drops its tier marker), `--numeric` stores the value as an unquoted YAML number (`bip: 4`; integers and finite floats only — `NaN`/`inf` and anything non-numeric are rejected), `--bool` stores an unquoted YAML boolean (`flag: true`; only `true`/`false` case-insensitive — `yes`/`no`/`1`/`0` are rejected), and `--encrypted` stores a value that is already a `🔒:v1` token verbatim. `--numeric`/`--bool` imply plaintext (numbers and booleans are never encrypted); all three conflict with `--encrypted`, `--numeric`/`--bool` conflict with each other, and `--plain` may ride along with either (the typed flag wins — numeric beats bool). Literal-mode values are shown unredacted in the confirm preview — only encrypted tokens are masked. Resulting YAML per mode:

```bash
dc config set app port --numeric --value 8080 --yes   # → port: 8080
dc config set app port --numeric --value=-3 --yes     # → port: -3 (negative needs `=`: `--value -3` parses as a flag)
dc config set app bip --numeric --value 1e3 --yes     # → bip: 1e3 (float notation accepted, stored as written)
dc config set app debug --bool --value TRUE --yes     # → debug: true (normalized to lowercase)
dc config set app bip --plain --value 4 --yes         # → bip: "4" (quoted string, unmarked)
dc config set app bip --value 4 --yes                 # → bip: "🔒 🔒:v1:…" (encrypted by default)
```

**Generate an auto value.**

```bash
dc get app db.password --auto password 32     # generate on miss; NOT persisted to the config
dc gen-secrets --file secrets/dc.local.yaml --inline   # fill every 🎲 placeholder in place
```

`--auto` memoizes the first draw only when an `--override`/`--fallback` env var was named — as a plaintext `export` line appended to `<project>/secrets/.envrc.auto`, which nothing sources. Durable values still go through `dc config set` / `dc set`.

**Override locally.** Per-machine value: `dc set` (lands in `local.yaml`). Per-environment: a `dev.yaml`/`staging.yaml` overlay selected by `$DC_ENV` (default `dev`). Per-invocation: `dc_export APP_DB_PASSWORD=app.db.password --override APP_DB_PASSWORD` (env wins when set) or `--fallback APP_DB_PASSWORD` (dc wins, env as fallback).

**Inspect safely.** Encrypted values print as `🔒 **redacted**` unless you reveal:

```bash
dc bat app db                        # masked YAML browse; --flat = file:line: paths, no values
dc secrets --sources                 # every in-scope key name (never values) + where it lives
dc compare app db.password --to "infisical:///app/db/DB_PASSWORD"   # digest-only ==/!=/??
STATUS=$(dc status); dc env --diff   # merge state; what changed vs the live environment
```

Capture plaintext only when needed, straight into a variable (audited; verify by length, never by printing):

```bash
DB_PASS=$(dc get app db.password --reveal --raw 2>/dev/null); echo "${#DB_PASS} chars"
```

**Round-trip / back up.** `dc emit` re-emits the store as loadable YAML (`secrets.dc.yaml` by default, `<out>.<layer>.yaml` variants carrying `_dc_layer`, exact `dc load` invocations printed for the special `_dc`/`auto` subjects). Tokens re-emit byte-identical, values are never printed, and targets must be `.gitignore`-covered unless `--force` — `dc load` on the emitted files reconstructs the same store.

### Pitfalls

- **`dc set` ≠ `dc config set`.** The first writes the store's `local` layer; the second splices your YAML. A forgotten store-local write silently outvotes every later edit of the load file (local 500 beats base 1000).
- **`--if-missing` seeds once.** A file loaded with it is never re-ingested on refresh — later edits to that file are invisible; its values are store-owned after the first load.
- **`subjects:` must be alone.** Wrapper plus other root keys is an ambiguity error; pick one shape per file.
- **Sequences replace, never merge.** Re-loading a file with a shorter list drops the old elements wholesale.
- **`--auto` is not persistence.** Without a named env var nothing is written anywhere (a fresh draw each miss); with one, the memo goes to `secrets/.envrc.auto` in plaintext and nothing sources it.
- **Env overrides read the live shell.** Under `--override ENV`, a stale exported variable in your current shell beats the freshly stored dc value (empty strings are ignored). `cd` out/in or `direnv reload` to re-evaluate.
- **No flatten rule, no env var.** Keys become env vars only via `_dc.flatten` rules (explicit or `prefix.*: PREFIX_*`) or the `_dc.export` list (`path.to.key` → `PATH_TO_KEY`); key name ≠ var name unless a rule says so. Wildcard rules skip `_`-prefixed keys.
- **The registry is append-only.** Each `dc load` re-registers its files in argument order (last = last loaded, and the insert target for new keys). Deleting a loaded file yields warnings, not failures; there is no deregister command yet (#15).
- **The secrets map is a separate mechanism.** `dc:`/`override:`/`auto:`/`default:` in `.infisical-secrets.yaml` are that file's per-var spec keys (precedence `ref → template → file → env → dc → fallback_dc → default`), not store layers — the fleet shorthand "layering dc: → override: → auto: → default:" refers to them. dc parses `auto:` there but never generates, and an entry with only `override:` has no `dc` source for `dc infisical get/set/compare` to round-trip.
- **Unmarked values print in the clear.** Bare `dc get` masks only `🔒` tokens and `⛔` sentinels; a secret stored without the marker comes back verbatim. Same for `dc get <subject>` with no path — the resolved snapshot prints with tokens still encrypted but plain values as-is. Keep every sensitive value `🔒`-marked.
- **Switching a 🔒 key to plain/numeric/bool replaces the stale secrets-layer copy.** Re-ingest drops the old encrypted entry instead of leaving it to shadow the new value (#40), so `dc get` returns the plain/typed value; older builds need `dc unset <subj> <path> --layer secrets` by hand.

### Quick reference (load workflow)

Full CLI list in [Command reference](#command-reference).

| Command | What it does |
|---|---|
| `dc load <file>... [--layer L] [--if-missing] [--replace] [--replace-key K] [--no-bump]` | ingest load YAML files (later files win); register in `.meta`; bump version once |
| `dc emit [--output F] [--subject S]... [--dry-run] [--force]` | inverse: store → loadable YAML; gitignore-gated, values never printed |
| `dc config get <name> <path>` | find the source line (`file:line`, redacted preview) |
| `dc config set <name> <path> --value V [--plain\|--numeric\|--bool\|--encrypted] [--yes] [-p] [--file F]` | encrypt + splice the owning source file, then refresh the store |
| `dc config setall <name> [--above/--below PATH]` | insert a YAML section from stdin |
| `dc set <name> <key> <value> [--layer L]` | store write (default layer `local`) — not a source edit |
| `dc gen-secrets [--file F] [--inline]` | fill `🎲` placeholders with generated, encrypted secrets |
| `dc secrets [--sources]` / `dc bat [name] [--flat]` | key names / masked browse — values never printed |
| `dc compare <name> <path> --to <target>` | digest-only compare against `infisical://` / `k8://` targets |
| `dc status` / `dc env --list` / `dc env --diff` | merge state / what would export / what changed |

`<name>` above is a **subject** — a named config in the store; see [Subjects](#subjects--what-dc-get-subject-means).

## Command reference

Run `dc --help` (or `dc <command> --help`) for full flags.

```bash
dc yaml <name>                     # merge YAML from stdin (--replace --replace-key --if-missing --layer)
dc load <file>...                  # ingest external YAML files; root keys = subjects (later files win)
dc get <name> [path]               # read a value (<name> = subject; --raw --override --fallback --auto --default --reveal --clippy)
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

`dc config set <subject> <path> --value X [--yes]` splices the owning `.envrc*`/registered file; `-p/--parents` builds missing parent sections (`--below/--above/--auto` place the new top-most section; alphabetical is the default with `-p`). Missing parents without `-p` fail with a hint. The default encrypts the value; `--plain` stores it literally with no 🔒 marker (non-secret values — overwriting an encrypted value drops its tier marker), `--numeric` writes an unquoted YAML number (`bip: 4` — integers/finite floats only, implies plaintext), `--bool` an unquoted YAML boolean (`flag: true`/`false` case-insensitive only, implies plaintext), and `--encrypted` stores an already-encrypted `🔒:v1` token verbatim (`--plain`/`--numeric`/`--bool` conflict with `--encrypted`; `--numeric`/`--bool` conflict with each other). Plain/numeric/bool values are shown unredacted in the confirm preview — masking applies only to encrypted tokens and 🔒/❗-marked lines.

`dc seal` — `--list [--json]` prints `key | file:line | state`; interactive selector on a tty, `--headless --encrypt/--decrypt KEY...` (or `--encrypt-all-plain`/`--decrypt-all`) otherwise. Repeatable `--match <PAT>` / `--except <PAT>` select by leaf key name only (case-insensitive wildcards: `*` any run of chars, `?` exactly one; `--except` wins; zero matches exits non-zero). `--file`, `--dry-run`, `--yes` compose in every mode. Store layer files are never rewritten and ⛔ shadow keys are never modified. Known limitation: multi-line block scalars collapse to one quoted line on rewrite.

## SDKs

Client libraries in Rust, TypeScript, Python, Elixir, and PHP share one API surface (`get_string`, `set`, …) for reading and writing dc stores from application code. `make sdk-build` / `sdk-test` / `sdk-publish` fan out to all five. See [sdk/README.md](sdk/README.md).

## Docs

- Architecture: [docs/PROJ-ARCH.md](docs/PROJ-ARCH.md) (digest: [PROJ-ARCH.summary.md](docs/PROJ-ARCH.summary.md)); internals in [docs/arch/](docs/arch/) — flatten rules, layer resolution, parent chain.
- Repo layout: [docs/PROJ-LAYOUT.md](docs/PROJ-LAYOUT.md) (digest: [PROJ-LAYOUT.summary.md](docs/PROJ-LAYOUT.summary.md)); per-directory notes in [docs/layout/](docs/layout/).
- More: schema ([docs/PROJ-SCHEMA.md](docs/PROJ-SCHEMA.md)), FAQ ([docs/PROJ-FAQ.md](docs/PROJ-FAQ.md)), howtos ([docs/PROJ-HOWTO.md](docs/PROJ-HOWTO.md), [docs/howto/](docs/howto/)), threat model ([docs/THREAT-MODEL.md](docs/THREAT-MODEL.md)), test health ([docs/TEST-HEALTH.md](docs/TEST-HEALTH.md)) — each has a `.summary.md` digest except TEST-HEALTH.
- `demo/` has a runnable walkthrough; `web/` is the Elixir/Hologram landing site, deployed via `helm/dirconfig-site`.
