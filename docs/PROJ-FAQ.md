# FAQ

Anticipated why/when/compared-to-what questions. For *how to* do something, see [PROJ-HOWTO.md](PROJ-HOWTO.md); for *how it's built*, see [PROJ-ARCH.md](PROJ-ARCH.md).

## Motivation

### Why would I use YAML files instead of plain `export VAR=value` in `.envrc`?

Because plain exports don't scale past a handful of variables and can't be updated by another process. A 50-line `.envrc` of `export` statements has no grouping, no layering (dev vs prod vs personal), and no way for a child script to change a value the parent shell will see — env vars are write-once from the parent's perspective. `dc` replaces that with named, nested YAML configs merged from `base → env → local → secrets`, so a deploy script can `dc set tab status "deploying"` and the parent shell picks it up on its next prompt. The trade-off: you now depend on a Rust binary and a shell hook instead of raw shell built-ins — for a project with 3 stable env vars and no cross-process update needs, plain `export` is still simpler and has zero moving parts.

→ *See [PROJ-HOWTO.md#how-to-migrate-a-raw-export-heavy-envrc-to-dc](PROJ-HOWTO.md#how-to-migrate-a-raw-export-heavy-envrc-to-dc).*

### Why store secrets in an encrypted `secrets.yaml` instead of just gitignoring a `.env` file?

Because gitignoring only protects secrets from *git* — it does nothing once the file is on disk, in a backup, or `cat`'d by accident, and it gives you no audit trail of who read what and when. `dc`'s secret values are individually encrypted (XChaCha20-Poly1305) as opaque scalars, redacted by default on every read, and every `--reveal` appends a JSONL audit record. The honest caveat: the encryption key itself lives in `~/.config/direnv-config/settings.yaml` on the same machine, so this defends against accidental exposure (screen shares, log dumps, `dc bat` output, careless `git add -A`) and gives forensic audit trail — it is not a substitute for a real secrets manager (Infisical, Vault) as the source of truth for a team; `dc` is designed to sit downstream of one.

→ *See [PROJ-HOWTO.md](howto/manage-secrets.md) and [PROJ-ARCH.md#secret-management](PROJ-ARCH.md#secret-management).*

### Why does a value need layering at all — why not one YAML file per config?

Because "the same key, different value per context" (dev vs prod cluster context, your personal tab theme vs the team's default, a secret vs its placeholder) is a recurring shape, and one flat file forces you to choose exactly one value or duplicate the whole file per environment. Layering (`base → {DC_ENV} → local → secrets`) lets the shared shape live in `base.yaml` (committable) while environment and personal deltas live in separate files that merge on top. The cost is one more concept to learn before your first `dc get` makes sense — for a single-environment project with no per-developer tweaks, a single `base.yaml` with no other layers is completely valid and adds no overhead.

→ *See [PROJ-HOWTO.md#how-to-layer-config-across-dev--prod--local--secrets](PROJ-HOWTO.md#how-to-layer-config-across-dev--prod--local--secrets), [arch/layer-resolution.md](arch/layer-resolution.md).*

## Fit

### When is `dc` the wrong tool for a project?

When the project has a handful of static env vars, no secrets worth encrypting, and no need for a child process to signal the parent shell — a plain `.envrc` with `export` lines is less to install and less to explain to a new contributor. `dc` earns its keep once you have layered environments, secrets that need redaction/audit, a parent/child directory hierarchy that should inherit config, or scripts that need to push live status back into the shell (tab titles, deploy state). If none of those apply, adopting it is pure overhead: a Rust binary, a shell hook, and a new mental model for no payoff.

### When should I use a monorepo's shared config instead of a per-project `dc` store?

`dc` stores are scoped per directory (with parent-chain inheritance), so a project *inside* a monorepo that already has an ancestor store (e.g. `k8/.envrc`) should extend that store by adding a child `.envrc`, not spin up an unrelated one — otherwise you lose inherited configs like `cluster` and `cloudflare` and end up duplicating them. Only start a fresh, unrelated store when the project genuinely has no relationship to the parent's config (a standalone tool checked into the monorepo for packaging convenience only).

→ *See [arch/parent-chain.md](arch/parent-chain.md), [PROJ-HOWTO.md#how-to-stop-a-subdirectory-from-inheriting-part-of-a-parents-config](PROJ-HOWTO.md#how-to-stop-a-subdirectory-from-inheriting-part-of-a-parents-config).*

## Comparison

### How is `dc` different from direnv's own layout/`.envrc` includes?

direnv gives you a file-inclusion and env-var-export primitive; it has no concept of named configs, layered merge, secrets, or cross-process writes. `dc` is built *on top of* direnv (via the `dc_yaml`/`dc_export` stdlib functions) rather than replacing it — you still need direnv installed and `.envrc` files allowed. Use plain direnv `source_up`/`export` when you only need file inclusion and static exports; reach for `dc` when you need merge semantics, secret handling, or IPC between processes.

### How does this differ from just using Infisical or Vault directly?

Infisical/Vault are the remote source of truth for secrets across a team and infrastructure; `dc` is a local, layered config cache with its own encryption that happens to have first-class Infisical/kubectl sync commands (`dc compare`, `dc push`, `dc infisical`) for keeping the two in agreement via SHA-256 digest comparison — no plaintext ever crosses that comparison. `dc` does not replace Infisical as the shared source of truth; it's the local-dev/runtime layer that resolves values (including non-secret config) into your shell and optionally reconciles with Infisical when asked.

→ *See [PROJ-HOWTO.md#how-to-compare-and-push-secrets-to-infisical--kubernetes](PROJ-HOWTO.md#how-to-compare-and-push-secrets-to-infisical--kubernetes).*

### How does `dc`'s "prune" differ from "purge"?

`prune` is the safe, git-friendly deletion: it writes a tombstone (`_dc_pruned: true`) that survives resolution and blocks inherited parent config from reappearing, but nothing is unrecoverably destroyed. `purge` is destructive-but-safe by contrast — it permanently deletes a named config or the whole store, with no tombstone, and is meant for hard cleanup (e.g. wiping a test store), not for routine "stop inheriting this from my parent" use.

→ *See [PROJ-HOWTO.md#how-to-stop-a-subdirectory-from-inheriting-part-of-a-parents-config](PROJ-HOWTO.md#how-to-stop-a-subdirectory-from-inheriting-part-of-a-parents-config).*

## Capability

### Can two shells/scripts write to the store at the same time safely?

Yes, as of the `m8-session-safety-hardening` milestone — every read-modify-write command (`set`/`yaml`/`unset`/`prune`/`purge`/`bump`) takes an exclusive `flock` on the store's `.lock` file, so concurrent invocations serialize rather than racing, and a layer file that fails to parse is left untouched instead of being silently replaced with an empty mapping. The caveat: the lock only protects `dc`'s own commands — hand-editing `base.yaml`/`local.yaml`/`secrets.yaml` directly while `dc` is mid-write is not covered by any lock.

→ *See [PROJ-HOWTO.md#how-to-recover-from-a-concurrent-write-conflict](PROJ-HOWTO.md#how-to-recover-from-a-concurrent-write-conflict).*

### Can a non-Rust app (Node, Python, Elixir, PHP) read `dc` config without shelling out?

Yes — SDKs exist for TypeScript, Python, Elixir, and PHP with a "native" backend that reads the resolved YAML directly (no `dc` binary required at runtime), plus a CLI backend that shells out for compatibility. All five SDKs (including Rust) share a contract-test suite that keeps encrypted-secret passthrough behavior identical across languages — none of the read-only SDKs decrypt secrets; they pass the opaque `🔒:v1:...` scalar through untouched.

### Can I get a value from a config without decrypting it, even to just inspect structure?

Yes — `dc bat` renders any config with secrets masked, and `dc bat --all --flat --filter-key <regex>` gives a line-numbered, dotted-path listing with no values at all, safe to grep or pipe into a doc-writing tool. This is intentional: `--flat` is for *locating* paths, not reading content, so it never leaks even a redacted placeholder.

→ *See [PROJ-HOWTO.md#how-to-search-or-audit-config-across-every-store-without-exposing-secrets](PROJ-HOWTO.md#how-to-search-or-audit-config-across-every-store-without-exposing-secrets).*

## Caveats

### What happens if the machine holding the encryption key is lost or wiped?

Every secret in `secrets.yaml` becomes permanently unreadable — `dc` has no key-recovery or key-escrow mechanism built in. The key lives at `~/.config/direnv-config/settings.yaml`, outside the versioned store, and is not backed up by anything `dc` does automatically. Treat it like any other local secret material: back it up yourself (password manager, encrypted backup) if losing local secrets would be costly, or keep Infisical/Vault as the actual source of truth and treat the local `dc` copy as disposable/re-derivable via `dc infisical`/`dc push`.

### What's the cost of the `precmd` shell hook — does it slow down every prompt?

It adds one cheap check per prompt: comparing the store's `.version` counter against the last-seen value, and only re-running `dc env` (a YAML read + flatten pass) when the counter actually changed. In the common case (no writes since the last prompt) the overhead is a single file stat/read, not a re-resolve. If you have many stores active in nested shells simultaneously, each contributes its own check, but no filesystem watcher or polling loop runs between prompts — it's evaluated once per `precmd`.

### Does deep-merging config across layers ever silently lose data?

Yes, in one specific case worth knowing before you rely on it: `dc yaml NAME --replace` wipes the *entire* named config's layer, not just the keys in the heredoc you pass — if a parent defined five keys and you `--replace` with one, the other four are gone. Ordinary `dc_yaml`/`dc set` calls deep-merge and don't lose sibling keys; only `--replace` (or `--replace-key KEY`, scoped to one branch) is destructive by design. If you're not sure which behavior you want, `--replace-key` is the safer default for "I want to overwrite one branch."

→ *See [PROJ-HOWTO.md#how-to-layer-config-across-dev--prod--local--secrets](PROJ-HOWTO.md#how-to-layer-config-across-dev--prod--local--secrets).*

### Why does the tab title only update on the shell's next prompt instead of pushing instantly?

Because `dc set tab ...` writes to a file the parent shell *polls*, not a socket it pushes over — there's no daemon and no reverse channel between the child script and the parent shell. The `precmd` hook checks the store's `.version` counter once per prompt and only re-resolves when it changed, so the visible update always lags to "whenever the next prompt draws," never the instant `dc set` runs. That's a deliberate trade for simplicity: two independent processes touching a shared file, no socket, no daemon, nothing to clean up if a script crashes mid-run. The honest cost: a long-running command with no natural prompt in between (e.g. a background script that sleeps and writes once at the end with no other shell interaction) won't visibly update until you press Enter or a new prompt draws — if you need true real-time feedback, this IPC isn't built for that and printing status directly to the terminal is the better tool.

→ *See [PROJ-HOWTO.md#how-to-push-a-live-status-update-from-a-script-tab-title-ipc](PROJ-HOWTO.md#how-to-push-a-live-status-update-from-a-script-tab-title-ipc).*

### Why do complex paths like `key[*].field` require `yq` installed instead of working out of the box?

Because `dc`'s own path resolver only implements simple dot-paths and single bracket indices, and wildcard/complex query paths are delegated to `yq`'s already-mature path-expression engine rather than reimplemented in Rust. This keeps the core binary small and avoids maintaining a second YAML-query parser alongside `yq`'s. The cost: a `key[*].field`-style lookup fails with an install hint instead of degrading gracefully when `yq` isn't present — plain dot-paths and single indices (`node_pool.min`, `platforms[0]`) never need it, so most day-to-day use is unaffected.

→ *See [PROJ-HOWTO.md#how-to-read-and-write-a-config-value-day-to-day](PROJ-HOWTO.md#how-to-read-and-write-a-config-value-day-to-day).*

## Trust

### Does anything ever leave my machine without me running a sync command explicitly?

No — `dc`'s core read/write/merge/env path is entirely local (files under `~/.local/state/direnv-config/`); network calls only happen when you explicitly invoke `dc compare`, `dc push`, or `dc infisical`, and even those compare SHA-256 digests rather than transmitting plaintext values by default. There's no background daemon phoning home.

### If I `dc get --reveal` a secret, is that logged anywhere I should worry about?

Yes, on purpose — every reveal (`--reveal` or the stricter `--reveal-restricted` for shadowed/`⛔` secrets) appends a JSONL record to the audit log (`audit.rs`, default path in the state dir, configurable). This is a feature, not a leak: it's what lets you answer "who looked at this secret and when" later. If that audit trail itself is sensitive (e.g. it could reveal which secrets exist and how often they're accessed), treat the audit log file with the same care as the state directory itself — it isn't encrypted the way secret values are.

→ *See [PROJ-ARCH.md#secret-management](PROJ-ARCH.md#secret-management).*
