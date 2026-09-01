# How-To Guides

Task-oriented guides for the things you'll actually do with `direnv-config`. For *what it is*, see [PROJ-ARCH.md](PROJ-ARCH.md); for *where things live*, see [PROJ-LAYOUT.md](PROJ-LAYOUT.md).

## How to: install direnv-config and verify it works
Get `dc` on your PATH, wired into direnv, and confirm a live `.envrc` exports env vars from YAML.
→ *See [howto/first-hour.md](howto/first-hour.md)*

## How to: migrate a raw `export`-heavy `.envrc` to dc
Replace 20-50 lines of `export VAR=value` with a structured, named YAML config `dc` manages.
→ *See [howto/migrate-envrc.md](howto/migrate-envrc.md)*

## How to: read and write a config value day to day

**Goal:** get/set a single value in a named config without hand-editing YAML.
**Prereqs:** a store already initialized in the current directory ([first-hour.md](howto/first-hour.md)).

```bash
dc get cluster name                  # resolved (merged) value
dc get cluster node_pool.min         # dot-path into nested map
dc get build platforms[0]            # bracket index into array
dc set tab status "deploying"        # writes to tab/local.yaml, bumps version
dc set --layer base cluster name "noizu"   # target a specific layer explicitly
```

**Verify:** `dc get <name> <path>` immediately reflects the write.
**Gotchas:** `dc set` with no `--layer` writes to `local.yaml` (personal/IPC layer), not `base.yaml` — if you meant to commit the change, add `--layer base`. Complex paths like `key[*].field` need `yq` installed; without it `dc get` errors with an install hint instead of silently failing.

## How to: layer config across dev / prod / local / secrets

**Goal:** keep one value's shape stable while environment-specific overrides and personal tweaks live in separate files.
**Prereqs:** understand the merge order — `base.yaml → {DC_ENV}.yaml → local.yaml → secrets.yaml` (see [arch/layer-resolution.md](arch/layer-resolution.md)).

```bash
dc yaml cluster <<'YAML'                     # base.yaml — shared, committable
name: noizu
YAML

dc yaml cluster --layer dev <<'YAML'          # dev.yaml — env overlay
context: noizu-dev
YAML

dc set tab theme catppuccin                   # local.yaml — personal override (default for dc set)
```

**Verify:** `dc get cluster` shows the deep-merged result; `dc get --env prod cluster` resolves against `prod.yaml` instead of `$DC_ENV`.
**Gotchas:** `dc yaml NAME --replace` wipes the *entire* named config's layer, not just the keys you pass — use `--replace-key KEY` to replace one branch while preserving siblings.

## How to: tag, read, and rotate secrets safely
Keep sensitive values encrypted at rest, redacted by default, and audited on every reveal.
→ *See [howto/manage-secrets.md](howto/manage-secrets.md)*

## How to: search or audit config across every store without exposing secrets

**Goal:** grep-style search over config keys and structure — for audits, doc-writing, or finding where a value is set — with secrets always masked.
**Prereqs:** none beyond an existing store.

```bash
dc bat --all                                  # every config in the resolved chain, secrets masked
dc bat cf                                     # one named config
dc bat cf access                              # a scope within it
dc bat --all --flat --filter-key 'token'      # line-numbered dotted-path list, no values — safe to pipe/grep
```

**Verify:** output pipes through `bat` when installed (syntax highlighting); falls back to plain text otherwise.
**Gotchas:** `--flat` intentionally omits values (even redacted placeholders) — it's for locating *paths*, not inspecting content; pair with `dc get <name> <path>` to read a specific value once you've found it.

## How to: push a live status update from a script (tab title IPC)

**Goal:** have a build/deploy script update the parent shell's tab title/theme while it runs, even though it's a separate process.
**Prereqs:** shell hook installed (`make install` / `dc-init`, see [first-hour.md](howto/first-hour.md)); [tabbing-on](../tabbing-on/) for the visual effect (optional — `dc set` works without it).

```bash
dc set tab status "building $APP"
dc set tab emoji rocket
dc set tab urgency 2
# ... do the work ...
dc set tab status "deployed $APP"
dc set tab emoji check
```

**Verify:** the parent shell's tab title updates on its *next prompt* (the `precmd` hook polls `.version`, not instantly).
**Gotchas:** if nothing updates, the shell hook isn't installed in this shell — `grep dc-init ~/.zshrc` and open a fresh terminal; `dc set` still succeeds (the write lands in `tab/local.yaml`) even with no hook watching it.

## How to: stop a subdirectory from inheriting part of a parent's config

**Goal:** a child directory's store inherits every named config from its parent by path; drop or replace what doesn't apply here.
**Prereqs:** a parent directory with an existing store (see [arch/parent-chain.md](arch/parent-chain.md)).

```bash
dc prune cluster                       # remove an entire inherited named config (writes a tombstone)
dc prune cloudflare tunnel             # remove just the `tunnel` branch within cloudflare
dc unset cloudflare zone_id_trl        # remove a single inherited key
dc yaml cloudflare --replace <<'YAML'  # full replacement instead of merge/prune
account_id: staging-cf-account
YAML
```

**Verify:** `dc env` (or a fresh shell) no longer exports vars for the pruned config; `dc get cloudflare` shows only what survived.
**Gotchas:** `dc prune NAME` deletes/tombstones — for permanent, unrecoverable removal (no tombstone, e.g. cleaning up a test store) use `dc purge NAME` (or `dc purge` with no name to wipe the whole store) instead.

## How to: recover from a concurrent-write conflict

**Goal:** understand what protects you when two shells/scripts write to the same store at once, and what to do if a write still looks lost.
**Prereqs:** none — this is built in as of the `m8-session-safety-hardening` milestone.

Every read-modify-write command (`set`/`yaml`/`unset`/`prune`/`purge`/`bump`) takes an exclusive `flock` on the store's `.lock` file, so simultaneous invocations serialize instead of racing. A layer file that exists but fails to parse is left untouched rather than silently replaced with an empty mapping.

```bash
dc status              # confirms current version + which layers are present
dc bat --all           # eyeball the current merged state before writing again
```

**Verify:** after a batch of concurrent writes (e.g. several tabs updating `tab` status), `dc status` version count matches the number of writes issued.
**Gotchas:** the lock only protects `dc`'s own commands — hand-editing a layer YAML file directly while `dc` is mid-write is not covered; always write through `dc set`/`dc yaml`, never edit `base.yaml`/`local.yaml`/`secrets.yaml` in place.

## How to: compare and push secrets to Infisical / Kubernetes
Confirm a local secret matches (or force-update) its remote copy, without ever printing plaintext.
→ *See [howto/sync-secrets-remote.md](howto/sync-secrets-remote.md)*

## How to: read (and write) `dc` config from app code without shelling out
Call `dc`-managed config directly from Rust, TypeScript, Python, Elixir, or PHP — no subprocess required.
→ *See [howto/use-sdk-from-app-code.md](howto/use-sdk-from-app-code.md)*
