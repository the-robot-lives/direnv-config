# Source ↔ Store Sync (load / emit / config)

`dc` keeps two representations in step: **source files** in the project (human-editable, splittable into git) and the **store** under `~/.local/state/direnv-config/` (resolved, encrypted at rest). This file covers the discovery of sources, the `dc load`/`dc emit` roundtrip, and the `dc config` line-splice editors.

## Source Inventory

Two kinds of source feed a store:

1. **`.envrc*` heredocs** — `dc_yaml <name> <<'YAML' … YAML` blocks in `.envrc` and `.envrc.*` siblings (discovered non-recursively in the project dir by `envrc/locator.rs`). `dc init --from-envrc` can scaffold a store from an existing `.envrc`.
2. **Registered external files** — YAML files ingested once via `dc load`, recorded in the store's `.meta` with absolute path, mtime, size, the `--layer` default used at load time, and an `if_missing` marker (such files seed the store once, then the store owns the values — refresh skips them).

`.meta` also records the managed source directory, creation time, config list, and an optional parent pointer (the chain walk itself is name-based — see arch/parent-chain.md).

## `dc load` (`src/cmd/load.rs`)

Input: one or more YAML documents, argument order = precedence (later files win on conflict, per-layer deep merge). Each document's root mapping maps **subject** names to config bodies; a lone `subjects:` wrapper key is unwrapped (mixed with other root keys → refused, not guessed). Per subject:

- A `_dc_layer` key on the body selects that subject's layer (the key is stripped at ingest); `--layer` is the default for subjects without one; `--replace`/`--replace-key`/`--if-missing` mirror `dc yaml`.
- The body runs the exact `dc yaml` pipeline: split-secrets walker encrypts `🔒` values into `secrets.yaml` (provenance routing — integration-tested), plain keys land in the chosen layer, bare tokens pass through verbatim.
- Unregistering is not automatic: files stay in `.meta` until pruned/purged, so `dc config` and `dc seal` keep finding their keys.

## `dc emit` (`src/cmd/emit.rs`) — the inverse

Re-emits the store's subjects as loadable YAML. Values are never printed; `--dry-run` prints counts only. Layer expressiveness: `_dc_layer` is per-subject routing, so one file can't place one subject in two layers — emission solves this structurally:

- **Main file** (`secrets.dc.yaml` default, `--output` to change): base-layer content with the secrets layer's tokens inlined (tokens need no routing — ingest auto-splits them).
- **Variant per other layer** (`<out>.<layer>.yaml`): that layer's content, each subject body carrying `_dc_layer: <layer>`.
- **`<out>.secrets.yaml` variant** only when a secrets layer holds *plain* values (which auto-split can't route).
- **Special subjects** `_dc` (canonically `dc load --replace`) and `auto` (canonically `--if-missing`) get their own files, since load flags are invocation-level; the exact `dc load` invocation is printed for each.

`🔒` tokens re-emit byte-identical under the at-rest `🔒 <token>` marked form — **emit never needs the key**. `dc load` of the emitted files reconstructs the same store.

## The Gitignore Gate

`ensure_safe_output()` refuses (unless `--force`) any emit output path that could be committed:

1. `git ls-files` — a **tracked** target is refused even if gitignored.
2. `git check-ignore -q` — a target **not covered by `.gitignore`** is refused.
3. Git verification failing at all (not a repo, git error) is also refused — fail loud, not permissive.

Git pathspecs are cwd-relative, so the gate resolves absolute outputs and `..` components to toplevel-relative form first. Rationale: emitted files can carry plaintext secret values; they must never land silently in a committable file.

## `dc config` Editors (`src/cmd/config.rs`)

- `dc config get <subject> <path>` locates where a key is set across `.envrc*` heredocs and registered files.
- `dc config set` encrypts (unless `--encrypted`) and splices the value into the owning source line. Editing reuses the locator's byte spans so comments/formatting survive and untouched lines re-emit byte-identical (`--yes` confirms; `--file` forces a destination validated against known sources).
- `-p/--parents` creates missing parent sections as nested mappings; the leaf carries the `🔒` marker/token exactly like a plain set. Placement of the new top-most section (mutually exclusive): `--below <PATH>` after the anchor's whole block, `--above <PATH>` before the anchor line, `--auto` alphabetical among siblings (the default with `-p`; appends last with no siblings). Without `-p`, a missing parent still errors (hinting at `-p`); the placement flags require `-p`.
- `dc config setall` inserts a whole YAML section above/below an anchor; `dc config secure` relocates a secret to the shadow store (see arch/secrets-and-sealing.md).

`dc seal` reuses this splice machinery for its in-place (un)seal rewrites.
