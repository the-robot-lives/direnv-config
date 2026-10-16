# Remote Sync

How `dc` moves secret material between the local store and remote backends without leaking it: Infisical (native API client) and Kubernetes Secrets (via `kubectl`).

## Target URIs (`src/target.rs`)

```text
infisical://<path>/<NAME>          # e.g. infisical://data/postgres/POSTGRES_PASSWORD
k8://<ns>/<secret>/<KEY>           # alias: kubernetes://<ns>/<secret>/<KEY>
```

Infisical paths may contain multiple `/`; the last segment is the secret NAME, the rest is the (leading-slash-normalized) secret path.

## `dc compare` / `dc push` (`src/cmd/compare.rs`, `src/cmd/push.rs`, `src/secretcmp.rs`)

- `dc compare <layer> <path> --to <target>…` hashes the local value and each remote value to SHA-256 digests and reports match/mismatch only. No plaintext, no length leak — the comparison is digest-only end to end.
- `dc push <subject> <path> --to <target>…` decrypts locally (internal API, no audit-free path: reveals by push are audited) and force-writes the remote value. `--dry-run` previews; `--yes` skips confirmation.
- Kubernetes writes go through `kubectl create/patch` on Secret objects (`src/kube.rs`); reads through `kubectl get`.

## `dc infisical` & the Secrets Map (`src/cmd/infisical.rs`, `src/secretsmap.rs`)

`dc infisical get|set|compare` is driven by the repo-root `.infisical-secrets.yaml` (apiVersion `secrets-tools/v1`) — the declarative mapping shared with the fleet's `infisical-populate-secrets` tooling. A secret not declared there won't resolve via `dc infisical`; use plain `dc compare`/`dc push` for ad-hoc targets.

Map shape: named **sections** (id + Infisical path) holding `vars` and `secrets` lists; each entry is a spec whose keys select the value source. `dc infisical set` edits the dc source line in the owning `.envrc*` in place; `dc infisical get` shows the live Infisical value next to its dc source (redacted unless `--reveal`).

## Resolution Precedence

`resolve_local` reproduces the populate-secrets order exactly — first matching source wins:

1. `ref` — another var in the map
2. `template` — `{var}` interpolation over already-resolved vars
3. `file` — file contents, optional `decode: base64`; `override:` env beats the file body
4. `env` — environment variable
5. `dc` — dc store lookup (decrypted), with `override:` env first and `fallback:` env after
6. `fallback_dc` — same, against a second dc path
7. `default` — static value

`auto:` is parsed for fidelity but never auto-generates here — resolve never invents values.

## Layering Prefixes vs the `dc get` Flag Chain

These are two different mechanisms and must not be conflated:

| | `.infisical-secrets.yaml` spec keys | `dc get` CLI flags |
|---|---|---|
| Where | per-var YAML spec (`dc:`, `override:`, `fallback:`, `auto:`, `default:`, …) | command-line flags (`--override ENV`, `--fallback ENV`, `--auto TYPE`, `--default VALUE`) |
| Resolved by | `secretsmap::resolve_local`, populate precedence above | `cmd/get.rs` run(): `--override` env → dc store → `--fallback` env → `--auto` → `--default` |
| `auto:`/`--auto` | parsed, never generates during resolve | generates on miss; at most memoizes the first draw to unsourced `secrets/.envrc.auto` — never enters the config |
| Scope | the map-driven `dc infisical` roundtrip | any single `dc get` |

The fleet shorthand "credential layering dc: → override: → auto: → default:" refers to the spec-key column. The `dc:` spec value itself is evaluated as a plain dc-store lookup (`get_secret_plaintext`) — the get-flag chain does not apply inside it.

## Infisical Client (`src/infisical.rs`)

Native Rust client — `dc compare`/`dc push` never shell out. Universal-auth login (client id/secret), Secrets v3, Folders v2; `reqwest` blocking with rustls TLS. Endpoint/config resolution checks the conventional env vars first (host, project, environment, client id/secret), then falls back to dc store lookups; optional Cloudflare Access service-token headers for fronted deployments. Plaintext travels only in request bodies — never to stdout, logs, or error messages. Values fetched for `dc infisical get` render redacted unless `--reveal` (audited).
