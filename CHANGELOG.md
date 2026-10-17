# Changelog — utilities/shell/direnv-config

## 2026-10-17
- feat(get): `dc get <subject> [<path>] --keys` (alias `--redact`) prints the YAML structure of the subject/section with every leaf VALUE replaced — 🔒 encrypted token, `<set>` non-empty plaintext (numbers/bools, `⛔` restricted sentinels), `<empty>` for `""`/null — never the value itself, never a `🔒:v1` ciphertext token; mapping keys, nesting, order and key quoting are preserved, pointers resolve keylessly, comments are dropped, and a scalar path prints `path: <marker>`. `--keys --flat` (alias `--paths`) prints one `subject.a.b[i]<TAB>marker` line per leaf (paths round-trip into `dc get` lookups). No decryption happens at all (works with no key available); `--keys` is rejected alongside `--reveal`/`--raw`/`--clippy`/`--reveal-restricted` with a clear clap error, and bare `--flat` requires `--keys`. Integration tests (`tests/get_keys.rs`, tempdir + redirected HOME/XDG_* + throwaway key): whole subject, nested section, list, single leaf, quoted keys, flat output, flag conflicts, no-key operation, and a plaintext/ciphertext leak sweep over stdout+stderr (#44)
- feat(get): `dc get <subject> <section> --reveal` decrypts EVERY encrypted leaf in the returned subtree (recursively, lists included) and prints the subtree YAML with structure/keys kept — previously the section dump printed children still as `🔒:v1` tokens and `--reveal` did nothing; plain leaves pass through, each decryption is audited per leaf, an undecryptable leaf aborts with a per-leaf error before anything is printed (no ciphertext on stdout), and without `--reveal` section output is byte-identical to before; `--raw` is a documented no-op on sections, `--clippy` copies the decrypted subtree (#42)
- fix(config): `dc config set -p/--parents` creates the FULL missing path — quoted keys (`"anthropic":`) are now indexed/unquoted for addressing and keep their quoting on rewrite (previously invisible to the locator, so `-p`/plain inserts created duplicate unquoted keys and corrupted the file), and an intermediate that already holds a scalar/flow value (`keys: "x"`, `keys: {}`) or a block scalar is refused with a clear error BEFORE any write (previously nested under the scalar, producing invalid YAML — or, for block scalars, silently appended INTO the string with a green exit); null sections (`keys:`) become mappings; same guard on the no-`-p` errata path (#41)
- fix(store): plain value replaces a stale 🔒 secrets-layer entry instead of being shadowed by it (#40)
- feat(config): `dc config set --numeric` / `--bool` for typed values; plain values shown unredacted in previews (#39); README examples
- fix(tests): clippy -D warnings in config_set_parents (#38)
- feat(config): `dc config set --plain` stores literal values without encryption (#37)
- docs(readme): in-depth guide to dc load yaml secret configuration
- docs(readme): explain dc subjects (secrets/services/apps/auto/_dc) + subject cheat sheet; widen secrets row
- Docs: refresh PROJ-LAYOUT (+layout/web.md), rewrite PROJ-ARCH (+arch/secrets-and-sealing, source-sync, remote-sync, ipc-model, sdk-backends), clean up README — direct to develop (markdown-only).

## [Unreleased]
- `dc config set -p/--parents` builds missing parent sections: intermediate keys of the path are inserted as nested YAML mappings (reusing the setall anchored-insert machinery — same file targeting, same 🔒 marker/token handling, same confirm/`--yes` flow), so `dc config set secrets inference.jev.api_key --value X -p --below sendgrid` works where the bare command previously bailed. Placement of the new top-most missing section among its siblings is exactly one of `--below <PATH>` (after the anchor's whole block), `--above <PATH>` (before the anchor line), or `--auto` (alphabetical among existing siblings at the deepest existing ancestor, appended at the end when there are none; also the default with bare `-p`). Without `-p` the missing-parent error now suggests `-p (with --below/--above/--auto) or dc config setall`, and the placement flags are rejected without `-p` (they only matter when a section is actually missing; when every parent already exists `-p` simply honours the placement for the new leaf). Values are never echoed — previews reuse the existing `**redacted**` masking. Unit tests (`parents_compute`: deep/below/above/auto first-middle-last/no-siblings/partial-prefix/anchor-mismatch/external-file) + integration tests against sandboxed temp fixtures (`tests/config_set_parents.rs`, throwaway key file + `XDG_STATE_HOME` in a tempdir; asserts the plaintext value never reaches stdout/stderr/files).
- Bugfix: `dc load`/`dc yaml` no longer double-encrypt a bare `🔒:v1:…` token (padlock with no following space) — `parse_marker` stripped the padlock before the already-encrypted check, so an at-rest token pasted into loadable YAML was re-encrypted into an unresolvable double wrap; the ingest walker (`secret::walk_split`) now recognizes a scalar that already *is* a token before marker handling and stores it verbatim (the safe `🔒 <token>` marked form behaves exactly as before; emit/seal/config round-trips unchanged).
- Seal selectors: `dc seal` gains repeatable `--match <PAT>` / `--except <PAT>` policy selectors for selective (un)sealing — ids/names/emails can stay plain while secrets/passwords/credentials encrypt. Patterns test the LEAF KEY NAME only (last dot-segment: `db_password` of `app.db.password`), case-insensitive and anchored, glob-style std-only syntax: `*` = any run of characters (incl. none), `?` = exactly one char, all else literal — no regex dependency. Selection starts from the action's base candidate set (plain keys for `--encrypt-all-plain`, sealed for `--decrypt-all`, or the explicit `--encrypt/--decrypt KEY` list), keeps entries matching any `--match` (OR), then removes every entry matching any `--except` — `--except` always wins. Selectors apply in every mode (`--list` inventory filter, TUI pre-filter, headless explicit + bulk) and compose with `--dry-run` (previews exactly the selected set) and `--json` (action summary now carries `selected` count + `selected_keys` names). Matching zero keys exits non-zero (`0 keys selected — check your patterns`) so a mis-typed pattern can never report success on an empty selection; ⛔ shadow keys and already-sealed keys remain untouchable by `--encrypt`.
- Review-hardening follow-up (#29): `dc emit --output` into a gitignored but not-yet-created nested directory now passes the safety gate without `--force` — the gate canonicalizes the nearest existing ancestor and resolves the nonexistent remainder lexically instead of falling back to a raw absolute pathspec that git rejects — and the write creates missing parent dirs; `dc seal` refresh dedup switched from `Vec::contains` to a `BTreeSet` membership check (O(n log n), op-order iteration preserved).
- `dc emit` — the inverse of `dc load`: re-emits every store subject as a loadable YAML file (`secrets.dc.yaml` by default, `--output` to change, `--subject` to filter to a subset of subjects). 🔒 tokens re-emit byte-identical (emit never needs the key), plain values stay plain, and non-base layers land in `<out>.<layer>.yaml` variants carrying per-subject `_dc_layer` routing (the `secrets` variant appears only when a subject stores plain values in its secrets layer, since tokens auto-split on ingest). Special subjects `_dc` and `auto` get their own files with the exact `dc load --replace` / `--if-missing` invocations printed, because load flags are invocation-level and cannot ride in the file. Values are never printed; `--dry-run` prints subject/key counts only; and every target is refused (unless `--force`) when it is git-tracked or not `.gitignore`-covered, so plaintext secret values never land in a committable file silently. Round-trip parity is pinned by integration tests (emit → `dc load` into a second sandboxed store → per-key + `.active` byte parity).
- Seal tooling: `dc seal` — encryption inventory + in-place (un)seal over project scope (`dc_yaml` heredoc blocks in `.envrc*` + registered external files; store layer files are never rewritten). `--list` prints `key | file:line | state` (🔒 `t<N>` / `plain` / `⛔ shadow`, `--json` for machines); bare `dc seal` opens a dependency-free `/dev/tty` selector (ranges/`all`/`none`, names-only preview, explicit confirm) and non-tty runs fail pointing at `--headless --encrypt/--decrypt KEY…` (+ `--encrypt-all-plain`/`--decrypt-all`, `--file` filter, `--dry-run`, `--yes`). Edits reuse the `dc config set` splice machinery behind new atomic temp+rename writes, so surrounding formatting survives and untouched 🔒 tokens re-emit byte-identical; decrypt restores the original marker/tier when known (bare originals come back as `🔒 <value>` plaintext-at-rest), ⛔ shadowed keys are listed but never modified, and no value — plaintext or token — is ever printed.
- `dc secrets` now lists every key name in scope for the project — merged across `.envrc*` `dc_yaml` blocks, registered external files, and every store layer across the resolution chain (previously only `secrets.yaml`) — with `--sources` annotating each key's `file:line` / `store:<subject>/<layer>` origins and `--sources --json` emitting annotated entries; values are never read into the listing.
- Release ops: first production release of the dirconfig.therobotlives.com landing site (image `sha-9a1d25c`, bump `5ce55a9`). `build-push` had failed at its `REGISTRY_USER`/`REGISTRY_PASSWORD` guard — neither Actions secret existed on this repo; both were set from the `ops_registry` dc anchor (`secrets registry.ops_user`/`registry.ops_password`), validated against `ops.noizu.com/auth` with push scope before storing, and the failed run was re-run to completion. ArgoCD Application `dirconfig-site` applied (ns `infra`, auto-syncs `helm/dirconfig-site` @ `main` into ns `apps`); `main` → `develop` back-merge (#25, `26514da`) keeps `develop` from regressing the released tag.
- Docs: refreshed `PROJ-ARCH.md` / `PROJ-LAYOUT.md` and per-directory layout docs (`docs/layout/sdk.md`, `docs/layout/src.md`)
- Keys: external lockable key file (`$DC_KEYS_FILE` → `~/.config/direnv-config/keys`, 0600) replaces the `key:` embedded in settings.yaml; `dc keys status|migrate|lock|unlock`. Legacy settings.yaml keys read transparently; `migrate` verifies every store token round-trips via the new key file before stripping key material (backup `settings.yaml.pre-keys`). Fail-loud on unreadable/missing key file — never silent legacy fallback.

## [m8-session-safety-hardening] — 2026-07-07 — tag: `utilities-shell-direnv-config/m8-session-safety-hardening`
Milestone summary: concurrent shells and terminal tabs could corrupt or clobber each other's config state; this milestone closes both the write-path race and a tab-scoping bug.

### Added
- Exclusive advisory lock (`flock` on `<store>/.lock`) around every read-modify-write command (`set`/`yaml`/`unset`/`prune`/`purge`/`bump`), preventing lost writes/version bumps from simultaneous invocations

### Changed
- `dc-init` tabbing bridge now reads the session-scoped `${DC_TAB_NS:-tab}` namespace instead of a hardcoded global `tab` config, so a version bump in one tab no longer re-applies another tab's theme/status/emoji into every shell
- `tabbing-on` exports `DC_TAB_NS` in lockstep with `TABBING_DC_UUID` generation; new shells evict inherited `TAB_*` appearance and regenerate session identity
- All tab-related `dc` writes unified on `--layer base` (previously defaulted to `local`, shadowing `dc yaml --replace` saves so cleared keys stuck)
- `tabbing-off`/`emit_purge` now also unset `TAB_THEME_DATA`, `TAB_BG`, `TAB_MARQUEE`, `DC_TAB_NS`, `_TABBING_OWNER_PID`, `_TABBING_WAS_ACTIVE`

### Fixed
- Read-modify-write commands refuse to overwrite a layer file that exists but fails to parse (previously a corrupt/partial layer was silently replaced with an empty mapping, losing all its keys)
- BSD `date +%s%3N` prints a literal `3N`; output is now validated as numeric before writing `last_update`

### Removed
- ~150 lines of dead shell theme-data value functions duplicated by `src/theme_data.rs` (zero remaining shell callers)
- Dead `read_layer` in `yaml.rs`, folded into `store::load_layer`

## [m7-encryption-toolkit] — 2026-06-16 — tag: `utilities-shell-direnv-config/m7-encryption-toolkit`
Milestone summary: adds a first-class encrypt/decrypt/generate command family so callers (e.g. `infisical-populate-secrets`) can reach raw unencrypted values without hand-rolled shell wrappers.

### Added
- `dc decrypt` / `dc encrypt` commands
- `dc gen` / `dc gen-secrets` commands for value and bulk secret generation
- Supporting `get.rs`, `config.rs`, `crypto.rs`, `store/resolve.rs` changes to route encrypted values through the new commands

## [m6-bat-flat-and-secret-wrappers] — 2026-06-14 — tag: `utilities-shell-direnv-config/m6-bat-flat-and-secret-wrappers`
Milestone summary: first milestone landed post-subtree-import into the monorepo; makes `dc bat` scriptable and repairs the legacy secret-viewing wrappers it was meant to replace.

### Added
- `dc bat --flat` — line-numbered dotted-path output without values, for config-path search/audit
- `docs/secret-management.md` — full reference for the six secret-management use cases

### Fixed
- `secret-engine.sh` now installs to `~/.local/lib/`, fixing broken legacy shell wrappers (e.g. `infisical-view-dc`)
- `infisical-view-dc`: fixed `local` used outside a function and broken filter logic
- Makefile/`.gitignore` tweaks for the monorepo layout

## [m5-dc-revamp] — 2026-06-02 — tag: `utilities-shell-direnv-config/m5-dc-revamp`
Milestone summary: the largest single command-surface expansion — `dc` grows audit, comparison, structured config inspection, and Infisical integration, backed by a new SDK contract-test fixture suite.

### Added
- `dc bat`, `dc compare`, `dc config`, `dc infisical` subcommands
- `src/audit.rs` — audit trail support
- `sdk/contract-tests/` — expectations + fixtures (`secret-store` vault, `.meta`, `.version`) shared across SDK implementations
- `docs/PROJ-ARCH.md` initial architecture doc

### Changed
- `dc get` significantly reworked to support the new resolution/audit paths

## [m4-purge-and-multilang-sdks] — 2026-05-26 — tag: `utilities-shell-direnv-config/m4-purge-and-multilang-sdks`
Milestone summary: introduces destructive-but-safe config deletion (`purge`) alongside the first cross-language SDK scaffolding and CI/publish pipeline.

### Added
- `dc purge [name]` — permanently deletes a named config or entire store; hard-purge mode for clearing files
- Hidden `dc __complete-purge` + zsh completion with dynamic config-name lookup
- `purge` tombstone written to the `base.yaml` layer (survives `resolve_active` regeneration on shell reload); parent-chain awareness writes a blocking tombstone when a config is inherited from a parent store
- `src/cmd/secrets.rs` + `lib/direnv-stdlib.sh` secrets helpers
- SDK scaffolding for Elixir, PHP, Python, Rust, TypeScript (`sdk/<lang>/`) with per-SDK Makefiles
- `.github/workflows/ci.yml` and `publish-sdks.yml`
- `docs/PROJ-ARCH.md`, `docs/PROJ-LAYOUT.md`, `docs/arch/*` (flatten-rules, layer-resolution, parent-chain), `docs/layout/sdk.md`
- `LICENSE`

## [m3-secret-resolution-flags] — 2026-05-25 — tag: `utilities-shell-direnv-config/m3-secret-resolution-flags`
Milestone summary: `dc get` becomes the general-purpose value-resolution entry point used by secret-population tooling.

### Added
- `dc get` gains `--override`, `--fallback`, `--auto`, and `--default` flags for flexible value resolution
- `--auto` generates passwords/hex and persists to `secrets/.envrc.auto`
- `rand` dependency for auto-generation

## [m2-migration-tooling] — 2026-05-21 — tag: `utilities-shell-direnv-config/m2-migration-tooling`
Milestone summary: smooths the on-ramp from raw `.envrc` files to `dc`-managed config.

### Added
- Demo `.envrc` files and README documenting the migration path from raw envrc to dc

### Changed
- Makefile: `install-cli` safety check (skip if src/dst are the same file), added `dc-init` install step

## [m1-initial-release] — 2026-05-12 — tag: `utilities-shell-direnv-config/m1-initial-release`
Milestone summary: first working version of `dc` — a layered, YAML-backed config store for direnv environments.

### Added
- YAML-backed config store with layered resolution (`base` → `$DC_ENV` → `local` → `secrets`)
- Parent-chain resolution via store name prefix matching
- Commands: `yaml`, `get`, `set`, `unset`, `prune`, `env`, `bump`, `init`, `status`, `list`
- `direnv-stdlib.sh` integration (`dc_yaml`, `dc_get`, `dc_export`)
- `dc-init` shell hook for version-based cache invalidation
- Zsh completions
- `Makefile` with install/uninstall/check/doctor targets
