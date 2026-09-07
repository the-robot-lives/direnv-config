# Project Architecture — Summary

**direnv-config** is a YAML-backed configuration + secrets layer for direnv consisting of a Rust CLI (`dc`), shell integration hooks, and five SDK clients (TypeScript, Python, Elixir, PHP read-only; Rust read + write) kept aligned by shared contract tests.

## Components

- **Rust CLI (`dc`)** — Manages YAML config stores: write configs, read values, resolve layers, export as env vars; secrets (encrypt/decrypt/gen/bat), `.envrc*` source editing (`config`), remote sync (compare/push/infisical)
- **direnv stdlib (`lib/direnv-stdlib.sh`)** — Shell functions (`dc_yaml`, `dc_export`, `dc_get`, `dc_set`) used in `.envrc` files
- **Shell hooks (`bin/dc-init`, `bin/tabbing-on-step`)** — `precmd` hook that watches `.version` for IPC-driven env reload; Zellij pane-title helper
- **Completions (`completions/`)** — zsh (`_dc`) + bash (`dc.bash`) via `make install-completions` (`shell/dc.zsh` is a deprecated stub)
- **SDKs (`sdk/`)** — Five language clients with native + CLI backends; `contract-tests/` shared fixtures
- **Demo (`demo/`)** — Simulated project/k8 `.envrc` trees with expected-state fixtures

## Key Architectural Patterns

- **Layer resolution**: `base.yaml` -> `{DC_ENV}.yaml` -> `local.yaml` -> `secrets.yaml`, deep-merged into `.active`
- **Parent chain inheritance**: Stores form an implicit hierarchy by filesystem path; configs deep-merge ancestor-first
- **Tombstone pruning**: `_dc_pruned: true` discards inherited config at any level
- **Flatten rules**: `_dc` config maps YAML paths to env var names (explicit + wildcard)
- **File-based IPC**: Monotonic `.version` counter enables cross-process state sharing via `precmd` polling
- **Secrets**: `🔒`-marked scalars encrypted at rest (XChaCha20-Poly1305) as flat self-describing strings; redaction by default, audited reveal, `⛔` shadow store, digest-based remote comparison (no plaintext leaks)

## Noizu Ecosystem Fit

Standalone Rust project (own `make install`, no `k8-lib`) — a dependency of the wider utilities rather than a consumer: `.envrc.k8.dc` holds scalar build/deploy config for `.infra-config.yaml` tooling, and `dc infisical`/`compare`/`push` feed the Infisical → k8s Secret flow.

## Technology

Rust (clap, serde_yaml, anyhow, chacha20poly1305, reqwest+rustls), POSIX shell, YAML configs stored at `~/.local/state/direnv-config/`; key/settings at `~/.config/direnv-config/settings.yaml`.
