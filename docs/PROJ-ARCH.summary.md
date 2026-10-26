# Project Architecture — Summary

**direnv-config** is a YAML-backed configuration + secrets layer for direnv: a Rust CLI (`dc`) managing per-directory config stores, a direnv stdlib extension bridging stores into shell env vars, a `precmd` hook giving cross-process IPC via a monotonic version counter, five read/write SDK clients, and a small Hologram landing site. It owns `.envrc.dc`, the scalar-config/secret source of truth for the trl-infra fleet.

## Components

- **Rust CLI (`dc`)** — store ops (yaml/load/get/set/env/bump/init/status/list/purge), secrets (gen, encrypt/decrypt, seal, secrets, bat), source editing (config get/set/setall/unset/secure, emit), key management (keys), remote sync (compare/push/infisical)
- **Store engine (`src/store/`, `src/yaml/`)** — path-hash store naming, advisory locking, four-layer deep merge with tombstones, parent-chain inheritance, flatten rules, dot-path expressions
- **Shell layer (`lib/`, `bin/`, `completions/`)** — direnv stdlib functions, IPC hook generator, Zellij tab-title helper, zsh/bash completions
- **SDKs (`sdk/`)** — five language clients, each with native (direct file) and CLI (subprocess) backends, aligned by shared contract tests
- **Web (`web/`, `helm/dirconfig-site/`)** — Phoenix/Hologram landing site, containerized, deployed by its own Helm chart

## Key Architectural Patterns

- **Layer resolution**: per named config, `base.yaml` → env overlay → `local.yaml` → `secrets.yaml` deep-merge into an `.active` snapshot, regenerated on every mutation under an advisory lock
- **Parent-chain inheritance**: stores chain by path-derived names; ancestors merge oldest-first; a tombstone breaks inheritance
- **Flatten rules**: the `_dc` config maps YAML paths to env var names (explicit + wildcard); `dc env` emits escaped exports plus DC_ROOT/DC_VERSION/DC_ENV
- **Value precedence**: env override → dc store → env fallback → auto-generation → static default; `--auto` at most memoizes the first draw to an unsourced `.envrc.auto` file — it never enters the YAML config (durable values go through `dc config set`, whose `--generate KIND` mints without echoing). `dc get --keys` reads shape only — every leaf value replaced by a marker, no decryption at all
- **File-based IPC**: every write bumps `.version` atomically; the `precmd` hook re-evaluates `dc env` on change
- **Secrets**: `🔒`-marked scalars encrypt at rest into flat self-describing `🔒:v1` tokens (XChaCha20-Poly1305) that merge, flatten, and SDKs pass through opaquely; key in an external lockable 0600 file; redaction by default with audited reveals; `⛔` shadow store; ingest auto-routes secrets to `secrets.yaml` with verbatim (never double-encrypted) tokens
- **Source ↔ store sync**: `dc load`/`dc emit` roundtrip external YAML and the store (tokens byte-identical, no key needed); emit refuses git-tracked or non-gitignored outputs unless forced; `dc config set`/`unset` splice source lines under a parse gate (revert on mismatch), `-p` builds missing parents with placement control, and write previews mask every non-target value
- **Remote sync**: SHA-256 digest compare against `infisical://` and `k8://` targets (no plaintext or length leaks); `.infisical-secrets.yaml` drives map-based sync with populate-compatible precedence
- **SDK dual backends**: all five languages read and write; native backend needs no binary, CLI backend guarantees identical semantics; contract tests pin behavior including secret passthrough

## Noizu Ecosystem Fit

Standalone install (own `make install`, no k8-lib dependency) — a dependency of the fleet rather than a consumer: `.envrc.dc` holds scalar build/deploy config for `.infra-config.yaml` tooling, and the Infisical sync feeds the Infisical → k8s Secret pipeline. The credential layering `dc: → override: → auto: → default:` maps to `dc get` flags and `.infisical-secrets.yaml` spec keys.

## Technology

Rust (clap, serde_yaml, anyhow, chacha20poly1305, reqwest + rustls, sha2), POSIX/zsh/bash shell, YAML state at `~/.local/state/direnv-config/`, settings and key file at `~/.config/direnv-config/`, SDKs in Rust/TypeScript/Python/Elixir/PHP, web in Phoenix/Hologram, CI via GitHub Actions.
