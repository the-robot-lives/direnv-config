# How-To Task List

Companion index to [PROJ-HOWTO.md](PROJ-HOWTO.md) — task + one-line outcome only, no steps.

| Task | Outcome |
|------|---------|
| Install direnv-config and verify it works | `dc` on PATH, wired into direnv, live `.envrc` exports confirmed |
| Migrate a raw `export`-heavy `.envrc` to dc | Existing exports imported into a structured, named YAML config |
| Read and write a config value day to day | `dc get`/`dc set` round-trips a value through the right layer |
| Layer config across dev / prod / local / secrets | Environment overlays and personal overrides merge predictably |
| Tag, read, and rotate secrets safely | Secrets encrypted at rest, redacted by default, reveals audited |
| Search or audit config across every store without exposing secrets | `dc bat`/`--flat` locates keys/paths with values always masked |
| Push a live status update from a script (tab title IPC) | A child process updates the parent shell's tab title via `dc set` |
| Stop a subdirectory from inheriting part of a parent's config | `dc prune`/`dc unset`/`--replace` scope a child store precisely |
| Recover from a concurrent-write conflict | Understand the `flock`-based write serialization and its limits |
| Compare and push secrets to Infisical / Kubernetes | Local secret confirmed against or force-synced to a remote, no plaintext leaked |
| Read (and write) `dc` config from app code without shelling out | Rust/TypeScript/Python/Elixir/PHP SDKs read/write the store directly, no `dc` binary needed |
