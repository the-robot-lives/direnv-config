# Changelog

## Unreleased

### Fixed
- Concurrent `dc set`/`yaml`/`unset`/`prune`/`purge`/`bump` now take an exclusive
  advisory lock (`flock` on `<store>/.lock`) for the whole read-modify-write, so
  simultaneous invocations from different shells/tabs can no longer lose each
  other's writes or drop `.version` increments
- Read-modify-write commands refuse to overwrite a layer file that exists but
  fails to parse (previously a corrupt/partial layer was silently replaced with
  an empty mapping, losing all its keys)
- `dc-init` tabbing bridge now reads the session-scoped namespace `${DC_TAB_NS:-tab}`
  instead of a hardcoded global `tab` config, so a version bump in one tab no
  longer re-applies another tab's last-set status/theme/emoji into every shell
- `purge` tombstone now written to `base.yaml` layer instead of `.active` directly, so it survives `resolve_active` regeneration on shell reload
- Parent-chain awareness: purge detects configs inherited from parent stores and writes a blocking tombstone

## 2026-05-26

### Added
- `dc purge [name]` command — permanently deletes a named config or entire store
- Hidden `dc __complete-purge` subcommand for zsh tab completion of config names
- Zsh completion support for `purge` with dynamic config name lookup

## 2026-05-25

### Changed
- `dc get` gains `--override`, `--fallback`, `--auto`, and `--default` flags for flexible value resolution
- `--auto` generates passwords/hex and persists to `secrets/.envrc.auto`
- Added `rand` dependency for auto-generation

## 2026-05-21

### Changed
- Makefile: added `install-cli` safety check (skip if src/dst are same file), added `dc-init` install step
- Added demo `.envrc` files and README documenting the migration path from raw envrc to dc

## 2026-05-12

### Added
- Initial release
- YAML-backed config store with layered resolution (`base` → `$DC_ENV` → `local` → `secrets`)
- Parent-chain resolution via store name prefix matching
- Commands: `yaml`, `get`, `set`, `unset`, `prune`, `env`, `bump`, `init`, `status`, `list`
- `direnv-stdlib.sh` integration (`dc_yaml`, `dc_get`, `dc_export`)
- `dc-init` shell hook for version-based cache invalidation
- Zsh completions
- `Makefile` with install/uninstall/check/doctor targets
