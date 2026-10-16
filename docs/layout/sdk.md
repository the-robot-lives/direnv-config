# SDK Layout

Client libraries for `dc` stores. All five SDKs (Elixir, PHP, Python, Rust, TypeScript) both read and write. Each SDK shares the same API surface (native + CLI backends, client, merge, resolve, path, store, version) and validates against shared contract tests.

```
sdk/
├── contract-tests/                 # Cross-language contract fixtures
│   ├── fixtures/
│   │   ├── simple-store/           #   Single-config store (.meta, .version, cluster/.active)
│   │   ├── nested-store/           #   Multi-config store (.meta, .version, app/.active)
│   │   └── secret-store/           #   Secrets store with vault/ config (.meta, .version, vault/.active)
│   └── expectations.yaml           #   Expected results for contract test assertions
├── elixir/                         # Elixir SDK (:direnv_config)
│   ├── lib/direnv_config.ex        #   Public API module
│   ├── lib/direnv_config/          #   cli + native backends, client GenServer, merge, path, resolve, store, version, watcher
│   ├── test/                       #   Contract + unit tests
│   └── mix.exs                     #   Package manifest (+ .formatter.exs, .tool-versions)
├── php/                            # PHP SDK (noizu/direnv-config)
│   ├── src/
│   │   ├── Backend/                #   BackendInterface, CliBackend, NativeBackend
│   │   ├── Exception/              #   ConfigNotFound, StoreNotFound, DcException
│   │   ├── DcClient.php            #   Public API
│   │   ├── Merge.php / Resolve.php #   Deep merge + layer resolution
│   │   ├── PathExpression.php / Segment.php   # Path expressions
│   │   └── Store.php / Version.php #   Store resolution + version tracking
│   ├── tests/                      #   PHPUnit tests (contract, native, path, merge, resolve, store, version)
│   └── composer.json               #   Package manifest (+ phpunit.xml, .tool-versions)
├── python/                         # Python SDK (noizu-direnv-config)
│   ├── src/direnv_config/          #   cli + native backends, client, merge, path, resolve, store, version, py.typed
│   ├── tests/                      #   pytest tests (contract, native, path, merge, resolve, store, version)
│   └── pyproject.toml              #   Package manifest (+ uv.lock)
├── rust/                           # Rust SDK (crates.io: direnv-config — read + write)
│   ├── src/                        #   client, merge, meta, path, resolve, store, version (lib.rs entry)
│   └── Cargo.toml                  #   Package manifest
├── typescript/                     # TypeScript SDK (@noizu/direnv-config)
│   ├── src/                        #   index exports, client, cli + native backends, merge, path, resolve, store, types, version
│   ├── test/                       #   Vitest tests (contract, native, path, merge, resolve, store, version)
│   ├── package.json                #   Package manifest
│   └── tsconfig.json               #   Build/test config (tsup.config.ts, vitest.config.ts)
├── Makefile                        # Cross-SDK build/test/publish/clean
└── README.md                       # SDK overview and quick-start
```

## Architecture

Each SDK implements two backends:

- **Native** — reads YAML files directly from `~/.local/state/direnv-config/` (no `dc` binary required)
- **CLI** — shells out to the `dc` binary for resolution

All SDKs validate against `contract-tests/expectations.yaml` using the shared fixtures.
