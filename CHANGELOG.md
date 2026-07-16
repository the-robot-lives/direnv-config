# Changelog — utilities/shell/direnv-config

## [Unreleased]
- Docs: refreshed `PROJ-ARCH.md` / `PROJ-LAYOUT.md` and per-directory layout docs (`docs/layout/sdk.md`, `docs/layout/src.md`)

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
