# Project Schema — Persistence & Config Artifacts

> **No persistence layer.** `direnv-config` has **no database, no SQL schema, and no
> Liquibase changelogs**. All persistence is file-based: YAML stores on disk, heredoc
> blocks embedded in `.envrc` files, encrypted secret tokens, and remote address
> schemes (Infisical / Kubernetes). This document is the structure reference for
> those artifacts. **No secret values appear here — formats only.**

## Artifact Map

```mermaid
graph TD
    ENVRC[".envrc (source of truth)<br/>dc_yaml heredocs per named config"] -->|dc_yaml / dc_set writes| STORE["state store<br/>~/.local/state/direnv-config/&lt;store&gt;/"]
    EXTFILES["external YAML files<br/>dc load: subject→body / subjects: wrapper / _dc_layer"] -->|dc_load ingest + .meta registry| STORE
    ENVRC -->|🔒 tag| SECRETS["secrets.yaml layer<br/>🔒:v1 encrypted tokens"]
    ENVRC -->|⛔ sentinel| SHADOW["shadow store<br/>&lt;project&gt;/.secrets/restricted.config.yaml"]
    STORE -->|resolve: base→env→local→secrets| ACTIVE[".active snapshot"]
    ACTIVE -->|flatten rules (_dc)| ENVVARS["env vars<br/>dc env / dc_export"]
    SECRETS -->|compare/push| INFISICAL["infisical:// addresses"]
    SECRETS -->|compare/push| K8S["k8:// addresses"]
    SETTINGS["settings.yaml<br/>~/.config/direnv-config/<br/>audit_log override · legacy key"] -->|legacy AEAD key| SECRETS
    KEYFILE["keys file<br/>~/.config/direnv-config/keys (0600)"] -->|AEAD key (preferred)| SECRETS
    STORE --> AUDIT["audit.log (0600)"]
```

```plantuml
@startuml
skinparam linetype ortho

component ".envrc\n(dc_yaml heredocs)" as envrc
component "external YAML files\n(dc load registry in .meta)" as extfiles
component "state store\n~/.local/state/direnv-config/<store>/" as store
component ".active snapshot" as active
component "secrets.yaml\n(🔒:v1 tokens)" as secrets
component "shadow store\n.secrets/restricted.config.yaml" as shadow
component "settings.yaml\n(audit_log; legacy key)" as settings
component "keys file\n(external, lockable, 0600)" as keyfile
component "audit.log (0600)" as audit
database "Infisical" as infisical
database "k8s Secrets" as k8s

envrc --> store : dc_yaml / dc_set
extfiles --> store : dc_load / dc config set (splice + re-ingest)
envrc --> secrets : 🔒 tag → encrypt
envrc --> shadow : dc config secure (⛔ sentinel)
store --> active : base→{DC_ENV}→local→secrets
active --> envrc : dc env / dc_export
secrets --> infisical : compare / push
secrets --> k8s : compare / push
settings --> secrets : legacy key (fallback)
keyfile --> secrets : XChaCha20-Poly1305 key (preferred)
secrets --> audit : --reveal events
@enduml
```

## `.envrc` — source of truth

Config source lives **inside `.envrc` heredoc blocks**, one per *named config*
(subject). There is no separate `.envrc.dc` file in this project.

```bash
# .envrc (structure)
dc_yaml <name> <<'YAML'
<key>: <value>              # plain scalars/maps
<key>: "🔒<secret>"         # 🔒 prefix → encrypted, routed to secrets.yaml
<key>: "🎲 <name> 🔒 hex 32" # 🎲 placeholder → dc gen-secrets fills in place
YAML

dc_yaml <name> --layer secrets   # target a specific layer explicitly
dc_prune <name> [<key>...]       # tombstone (_dc_pruned: true) a config/branch
dc_set <name> <key> <value>      # writes land in local.yaml (via IPC)
dc_bump                          # finalize after batched writes
```

| Marker | Meaning |
|--------|---------|
| *(none)* | plaintext, stored per `--layer` (default base) |
| `🔒` | secret — encrypt to `🔒:v1` token, route to `secrets.yaml` |
| `❗`×0–3 | sensitivity tier suffix on `🔒` (low → high) |
| `🎲 <name> 🔒 <charset> <len>` | generator placeholder for `dc gen-secrets` |
| `⛔` | restricted sentinel — value lives in shadow store only |

## External YAML files — `dc load`

Config sources may also live in **plain YAML files** outside `.envrc`, ingested
with `dc load <file> [<file>...]` (or the `dc_load` stdlib shim). Each file is a
single YAML document; the root mapping is **subject name → config body**. The
same markers/encryption as heredocs apply — a `🔒` value encrypts to a `🔒:v1`
token in `secrets.yaml`; nothing about the crypto changes.

```yaml
# team.yaml — root mapping = subject -> body
cluster:
  name: noizu
cf:
  access:
    client_secret: "🔒 hunter2"   # encrypted at ingest; never stored plaintext

# alternative: subjects: wrapper (same file, one style only — mixing the
# wrapper with other root keys is an error)
subjects:
  cluster:
    name: noizu
```

Per-subject routing key (stripped before ingest):

| Key | Meaning |
|-----|---------|
| `_dc_layer: <name>` | route that subject to a layer; the CLI `--layer` flag is the default for subjects lacking it; layer defaults to `base` |

`dc load` mirrors the `dc yaml` flags (`--replace`, `--replace-key`,
`--if-missing`, `--no-bump`) and bumps the version once at the end. When
several files are loaded in one invocation (or across invocations), **later
files win** on key conflicts (deep merge per ingest step).

### Provenance registry (store `.meta`)

Loaded files are registered in the store's `.meta` under `external:`, in load
order (last entry = last loaded), each `{path (absolute), mtime, size}`. The
registry drives lookup and write routing:

- `dc config get <subject> <path>` searches `.envrc*` **and** every registered
  external file, reporting `file:line`.
- `dc config set` splices the file that owns the key (comments and sibling
  lines preserved); a key found in **two** files refuses with every candidate
  listed — disambiguate with `--file <path>` (validated against registered
  files and `.envrc*` sources).
- **New keys** (errata) insert into the **last-loaded** file — unless
  `--file <path>` overrides — then fall back to earlier registered files and
  `.envrc*` heredocs when the subject is absent from the last-loaded one.
- A registered file that has vanished produces a **warning**, not a failure
  (re-run `dc load` or edit the registry to refresh it).

### Ingest precedence

Within one layer, **ingest order decides** — each ingest step deep-merges over
what is already in the store, so the last source to write a key per layer wins.
There is no "inline beats external" rule: a `dc_yaml` heredoc loaded *after* a
`dc load` file wins; loaded after, the external file wins. Across layers the
existing weights apply unchanged (`base` 1000 → env 750 → `local` 500 →
`secrets` 250 wins). `dc config set` re-ingests the subject's sources with the
just-edited file last, so the edit takes effect immediately.

## State store layout

One store per directory with an `.envrc`, named by path hash (strip leading `/`,
`/`→`-`; >200 chars truncated + 8-hex SHA-256 suffix). Under
`$XDG_STATE_HOME/direnv-config/` (default `~/.local/state/direnv-config/`):

```
<store-name>/
├── .version                  # monotonic integer counter
├── .meta                     # source abs path, timestamps, full hash, external-file registry
├── <config-name>/            # one dir per named config
│   ├── base.yaml             # committable defaults
│   ├── {DC_ENV}.yaml         # env overlay (DC_ENV defaults dev)
│   ├── local.yaml            # personal overrides / IPC writes (gitignored)
│   ├── secrets.yaml          # 🔒:v1 encrypted values (gitignored)
│   └── .active               # resolved merge snapshot
├── _dc/base.yaml             # special config: flatten rules, prefix, export list
└── history/NNN-<ISO8601>.yaml  # append-only snapshots
```

### Layer resolution order (per config)

`base.yaml` → `{DC_ENV}.yaml` → `local.yaml` → `secrets.yaml` → written to `.active`.
Missing layers skipped. Deep-merge: maps recurse, sequences/scalars replace,
`_dc_pruned: true` tombstones prune the subtree. Cross-store: parent chain
(`Users-…-k8` ← `Users-…-k8-projects`) merges ancestor-first; a root-level
`_dc_pruned: true` in a child breaks inheritance for that config.

**Note:** this layering is project-local. The monorepo's `dc:/override:/auto:/default:`
credential-prefix scheme is a consumer convention layered on top — `dc` itself
resolves only the four files above plus the parent chain.

## Encrypted token & shadow store formats

| Artifact | Format | Notes |
|----------|--------|-------|
| Secret token | `🔒:v1:<ciphertext>` | XChaCha20-Poly1305 AEAD; key held in the external key file (preferred) or legacy `key:` in settings.yaml. Key rotation orphans existing tokens (loud failure) |
| Key file | `~/.config/direnv-config/keys` (0600) | `{version: 1, key: <base64 32-byte>}`; path via `$DC_KEYS_FILE` → `$XDG_CONFIG_HOME/...` → default. Present ⇒ used exclusively (no silent fallback); root-ownable/lockable via `dc keys lock`, reversed by `unlock`; `dc keys migrate` moves a legacy key out of settings.yaml after a full decrypt round-trip verification |
| Shadow store | `<project>/.secrets/restricted.config.yaml` | Created by `dc config secure`; store + `.envrc` keep only a `⛔` sentinel; requires `--reveal-restricted` |
| Audit log | `$audit_log` or `~/.local/state/direnv-config/audit.log` | Append-only, mode 0600; records every `--reveal` / `--clippy` / `--reveal-restricted` |

## `settings.yaml` (global)

Path resolution: `$DC_SETTINGS` → `$XDG_CONFIG_HOME/direnv-config/settings.yaml`
→ `~/.config/direnv-config/settings.yaml`.

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `key` | string | No (legacy) | base64-encoded 32-byte XChaCha20-Poly1305 key. Superseded by the external key file; used only when that file is absent. Move via `dc keys migrate` |
| `audit_log` | string | No | Override audit-log path |

## Remote address schemes

Used by `dc compare` / `dc push` / `dc infisical` — values never printed.

| Scheme | Shape | Example |
|--------|-------|---------|
| Infisical | `infisical://<project>/<KEY_PATH>` | `infisical://cf/CF_API_TOKEN` |
| Kubernetes | `k8://<namespace>/<secret-name>/<KEY>` (`kubernetes://` alias) | `k8://apps-ns/cf-secrets/CF_API_TOKEN` |
| Infisical mapping | `/section/KEY` (against `.infisical-secrets.yaml`) | `dc infisical get POSTGRES_PASSWORD` |

Connection settings resolve from env vars, then `dc get secrets infisical.*` /
`dc get cf access.*` from the resolved configs.

## `.infisical-secrets.yaml` (secrets map)

Declarative mapping (apiVersion `secrets-tools/v1`) of Infisical secret names to
their `dc` lookup sources; drives `infisical-populate-secrets` and
`dc infisical compare/get/set`. Parsed by `src/secretsmap.rs`.

```yaml
apiVersion: secrets-tools/v1
sections:
  - id: <section-id>          # Infisical folder/section identity
    path: <infisical-path>    # destination path in Infisical
    vars:                     # plain vars
      <NAME>: <spec>
    secrets:                  # secret vars (same spec shape)
      <NAME>: <spec>
```

Per-name `<spec>` resolution precedence (first hit wins):
`ref` → `template` → `file` → `env` → `dc "<config> <path>"` →
`fallback_dc` → `default`. Optional flags: `decode`, `optional` (skip missing
instead of failing), `override_env`/`fallback_env` (env-var overrides),
`auto` (generation markers — parsed but never auto-generated by `dc`).

## Derived env vars

`dc env` (evaled by the `.envrc` helper `dc_export`) emits, driven by the `_dc`
config's `flatten:` rules (explicit `path: VAR` and wildcard `prefix.*: PREFIX_*`):

- Reserved: `DC_ROOT`, `DC_VERSION`, `DC_ENV`
- Mapped keys per `flatten`; unmapped keys export as `PREFIX_PATH_TO_KEY`
  (uppercase, dots→underscores) for keys listed in `_dc.export`
- Shell-special values are single-quote escaped

## Fixture contracts

`demo/expected-state/*.yaml` are the golden resolved outputs for the demo trees
(`root.yaml`, `projects.yaml`, `k8.yaml`, `staging.yaml`, `design.yaml`);
`sdk/contract-tests/expectations.yaml` pins cross-SDK read/write behavior. Both
must be updated alongside any resolution/merge semantics change.
