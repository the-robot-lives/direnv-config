# IPC Model — Shell Hook & Version Watching

How child processes propagate config changes into a parent shell without any socket or daemon.

## The Problem

A child process cannot modify its parent's environment. But because `dc` state lives in files, any process can edit the store — the remaining problem is telling the parent shell "something changed."

## `.version` Counter (`src/store/version.rs`)

- Every store directory holds a `.version` file containing a monotonically increasing integer.
- Every mutating command (ingest, set/unset/prune, config writes, SDK writes) re-resolves `.active` and calls `bump_version()` unless `--no-bump`.
- The bump is atomic: write to a temp file, then rename over `.version` — readers never see a torn value.

## The `precmd` Hook (`bin/dc-init`)

`eval "$(dc-init zsh)"` (or `bash`) emits a `_dc_precmd_hook` registered in `precmd_functions` (zsh) or `PROMPT_COMMAND` (bash). On every prompt it:

1. Reads `$DC_ROOT/.version` (silently does nothing if `DC_ROOT` is unset or the file is missing).
2. Compares against `_DC_LAST_VERSION`; on change, updates the memo and re-evals `dc env` — failures are swallowed (`|| true`) so a broken store never wedges the prompt.
3. Bridges to the Zellij tab-title tooling when loaded: reads `status`/`theme`/`emoji` from the config named by `DC_TAB_NS` (default `tab`), calling `tabbing-status`/`tabbing-style` only on actual value changes. Per-session namespaces (`DC_TAB_NS=tab-<uuid>`) keep each tab pulling its own state rather than the last value any other tab wrote.

## Communication Patterns

- **Child → parent**: script runs `dc set tab status "deploying"` (bumps `.version`); parent shell re-exports and updates its tab title on the next prompt.
- **Process → process**: two shells in the same directory share the store; both pick up each other's writes.
- **SDK → shell**: an application writes via an SDK backend; the shell sees the bump.

## Properties & Limits

- No daemon, no sockets, no watchers — survives crashes and works across unrelated processes.
- Latency is one prompt: changes land at the next prompt, not mid-command.
- `dc env` output is `eval`'d by design — anything that can write the store can inject env vars into every shell that resolves it (single-user workstation trust assumption; see THREAT-MODEL.md T-001).
- Exported anchor vars: `DC_ROOT` (store source dir), `DC_VERSION`, `DC_ENV` are emitted by `dc env` on every re-export.
