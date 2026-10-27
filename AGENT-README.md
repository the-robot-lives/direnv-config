# AGENT-README

> Orientation for AI coding agents working in this repo. Human docs: README.md.

## Type

rust-cli + scripts

## Purpose

YAML-backed configuration layer for direnv: the `dc` CLI (Rust) plus a direnv shell library replace sprawling `export VAR=value` `.envrc` files with structured, versioned, mergeable YAML named configs that flatten into env vars automatically.

## Stack

Rust CLI (`dc`, `src/` + Cargo), direnv shell library (`lib/`, shell hook, `dc_yaml`/`dc_load`/`dc_export` helpers), completions, Makefile. State lives under `~/.local/state/direnv-config/<path-hash>/`.

## Dev mode

```bash
make compile              # cargo build --release
make test
make install              # dc binary + dc-init + direnv lib, shell hook, completions
make doctor               # diagnose install issues (make check = quick verify)
```

## Ports / URLs

- none (library/CLI)

## Tests

```bash
make test
```

## Gotchas

- Layer merge order is fixed: `base.yaml` (committable) → `$DC_ENV` overlay → `local.yaml` → `secrets.yaml` (gitignored); weights configurable in `base.yaml`.
- `dc get --auto` generates on miss but does NOT persist the value — use `dc config set --yes` for durable values.
- Sealed secrets: `dc encrypt` wraps plaintext as `🔒:v1:t<tier>:…` tokens at rest; `dc decrypt` / `dc get --reveal` / `dc bat --reveal` read them back (audited).
- This repo owns `.envrc.dc` — the scalar-config/secret source of truth for trl-infra (AWS, Docker, Helm, Infisical creds).
