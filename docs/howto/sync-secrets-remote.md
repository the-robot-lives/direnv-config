# How to: compare and push secrets to Infisical / Kubernetes

**Goal:** confirm a local `dc`-managed secret matches (or force-update) its remote copy, without ever printing plaintext to stdout or a log.
**Prereqs:** network access to Infisical / the k8s cluster; Infisical connection settings resolvable via env vars or `dc get secrets infisical.*`; `kubectl` context set for Kubernetes targets.

1. Compare a local secret against one or more remote targets — output is match/mismatch only, no values:
   ```bash
   dc compare cf api_token \
     --to infisical://cf/CF_API_TOKEN \
     --to kubernetes://apps-ns/cf-secrets/CF_API_TOKEN
   ```
2. If a mismatch is found and the local copy is authoritative, preview the push (dry-run is the default — nothing is written):
   ```bash
   dc push cf api_token --to infisical://cf --dry-run
   ```
3. Confirm and apply:
   ```bash
   dc push cf api_token --to kubernetes://apps-ns/cf-secrets --yes
   ```
4. For the `.infisical-secrets.yaml`-mapped secrets specifically (the repo-root declarative mapping), use the `dc infisical` roundtrip instead of raw `compare`/`push`:
   ```bash
   dc infisical compare /data/postgres/POSTGRES_PASSWORD --to kubernetes://data/shared-postgres/POSTGRES_PASSWORD
   dc infisical get POSTGRES_PASSWORD                  # redacted diff: live Infisical vs dc source
   dc infisical set POSTGRES_PASSWORD --value "$NEW"   # locates the dc source, edits .envrc in place
   ```

**Verify:** re-run the same `dc compare`/`dc infisical compare` — it should report a match after a successful push.

**Gotchas:**
- `dc push` without `--dry-run` and without `--yes` still won't apply — both `--dry-run` (explicit) and omitting `--yes` are safe-by-default; you must pass `--yes` to actually write.
- Comparison uses SHA-256 digests, not plaintext — a "mismatch" only tells you the values differ, not how; use `--reveal` locally and check the remote system directly if you need to see why.
- `dc infisical` calls are driven by the `.infisical-secrets.yaml` mapping at the repo root — a secret not declared there won't resolve via `dc infisical get/set`; use plain `dc compare`/`dc push` for ad hoc targets instead.
