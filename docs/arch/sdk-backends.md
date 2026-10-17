# SDK Backends

Five language clients give application code `dc`'s resolution semantics without requiring shell integration — and, since all five implement writes, without requiring the `dc` binary at all when using the native backend.

## Packages

| Language | Package | Registry | Writes |
|----------|---------|----------|--------|
| Rust | `direnv-config` | crates.io | yes |
| TypeScript | `@noizu/direnv-config` | npm | yes |
| Python | `noizu-direnv-config` | PyPI | yes |
| Elixir | `:direnv_config` | Hex | yes |
| PHP | `noizu/direnv-config` | Packagist | yes |

All five share one API surface; the historical "only Rust writes" split no longer holds — every SDK's client exposes `set`/`unset` (and `bump`) backed by both backends.

## Shared API Surface

- `get` / `get_string` / `get_int` / `get_bool` (+ `get_or_raise` variants) over dot-path expressions (`node_pool.min`, `endpoints[*].host` — implemented per language in `path.*`).
- `list_configs`, `version`, `has_changed(since)` — store inventory and change detection.
- `set(config, key, value, layer?, noBump?)`, `unset(config, keys, …)`, `bump()` — writes default to the **`local`** layer and bump `.version`, honoring the IPC contract so shells pick changes up.
- Deep merge (`merge.*`) and layer resolution (`resolve.*`) mirror the Rust semantics: four-layer order, overlay-wins maps, sequence replacement, `_dc_pruned` tombstones.
- Version watching where the runtime supports it (`watchVersion` in TypeScript, a GenServer + `watcher` in Elixir) for push-style change notification instead of polling.

## Backends

Each client takes two interchangeable backends behind one interface (`Backend`/`BackendInterface`):

- **Native** — locates the store under `~/.local/state/direnv-config/` using the same path-hash naming, reads `.active` / layer files directly, and implements writes by editing layer YAML and bumping `.version` itself. No `dc` binary needed.
- **CLI** — shells out to `dc get`/`dc set`/`dc unset`/`dc bump`. Guarantees byte-identical semantics with the CLI at the cost of a subprocess; useful in environments where the state dir layout might drift.

Resolution (chain walk, tombstones) runs client-side in both backends: the CLI backend only performs single ops, so cross-store merging logic is duplicated by design and pinned by tests.

## Contract Tests (`sdk/contract-tests/`)

Fixtures (`simple-store`, `nested-store`, `secret-store` — complete on-disk stores with `.meta`/`.version`/`.active`) plus `expectations.yaml` asserting expected values per (store, config, path). Every language's test suite validates the same expectations, which is what keeps merge/resolve/path behavior from drifting. The secret-store fixture pins the **opaque passthrough** contract: `🔒:v1` tokens come back as untouched strings — no SDK ever decrypts or needs the key.

## Publishing

`sdk/Makefile` orchestrates build/test/publish across languages (`make sdk-build` / `sdk-test` / `sdk-publish` from the repo root). CI publishes on `sdk-v*` tags via `.github/workflows/publish-sdks.yml`; ordinary pushes run `ci.yml`.
