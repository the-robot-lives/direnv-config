# Secrets & Sealing

How `dc` marks, encrypts, reveals, and (un)seals secret values. Values are never printed by any command unless an explicit reveal flag is given, and every reveal is audited.

## Marking Scheme (`src/secret.rs`)

- A YAML scalar is a **secret** when its trimmed body begins with `🔒`.
- A run of `❗` immediately after the padlock sets the sensitivity tier: 0 (unmarked) up to 3; each `❗` after the third is ignored.
- The body after marker/tier is trimmed at the ends only — interior whitespace/newlines are preserved byte-exact through encryption.
- `⛔` (alone) is the **restricted sentinel** for shadowed values, handled by `src/shadow.rs`, never by the normal encrypt path.
- A scalar that already is a bare encrypted token (`🔒:v1:…`, no space) is recognized before marker handling and ingested **verbatim** — no double encryption (regression-tested since #32).
- `dc gen-secrets` walks a different marker: `🎲 handle [🔒] [encoding] [length]` placeholders generate values in place (encrypted when `🔒` present).

## Token Anatomy (`src/crypto.rs`)

```text
🔒:v1:t<tier>:<base64url( nonce(24 bytes) || ciphertext || tag )>
```

- XChaCha20-Poly1305 AEAD (RustCrypto `chacha20poly1305`); fresh random 24-byte nonce per encryption.
- The tier rides **inside** the token, so redaction fidelity survives every merge, copy, and SDK hand-off without the key.
- The flat single-scalar shape is load-bearing: `merge`, `path`, `flatten`, and every SDK treat it as an opaque string. `sdk/contract-tests` (secret-store fixture) enforces the passthrough.
- The legacy `dcenc:v1:` prefix is still recognized for decryption.

## Key Management (`src/keys.rs`, `src/settings.rs`)

- Preferred: the dedicated key file — `$DC_KEYS_FILE` → `$XDG_CONFIG_HOME/direnv-config/keys` → `~/.config/direnv-config/keys`. Schema `version: 1` + base64 (standard) 32-byte key, written 0600 via atomic temp+rename.
- Resolution: key file present ⇒ used **exclusively** (unreadable/invalid fails loud — never a silent legacy fallback); key file absent ⇒ legacy transparent read of `key:` in `settings.yaml`.
- `dc keys status` reports the active mode. `dc keys migrate` moves the legacy key into the key file only after decrypting every token in the target stores with the new file (round-trip verification) — then strips the key material from settings.yaml (`--strip-only` skips verification when the file already exists).
- `dc keys lock` prints (and with `--yes` runs via sudo) root-ownership hardening so the file becomes read-only to the user; `dc keys unlock` reverses it.

## Ingest Routing (`dc yaml` / `dc load`)

The split-and-encrypt walker separates a submitted tree into plain and secret entries. Plain keys land in the requested layer; every `🔒` value is encrypted and written to the subject's `secrets.yaml` **regardless of `--layer`** (the "provenance routing" integration test pins this). Bare tokens pass through verbatim. Every mutating ingest re-resolves `.active` and bumps `.version`.

## Reading & Reveal (`dc get`, `dc bat`)

- Default: a token renders as a tier-annotated placeholder (`🔒❗ **redacted**`); the key is never touched. Plain values pass through.
- `--reveal` decrypts to stdout; `--clippy` decrypts to the macOS clipboard (`pbcopy`) instead. Both append a JSONL audit record (`src/audit.rs`: subject, path, tier, method, user, timestamp — never the value) to `audit_log` from settings.yaml, else `~/.local/state/direnv-config/audit.log` (0600). Audit write failure warns on stderr but never blocks the operation.
- `⛔` values resolve through the shadow store and require `--reveal-restricted` (or `--clippy`); a redacted-restricted *attempt* is itself audited.

## Shadow Store (`src/shadow.rs`)

`dc config secure <subject> <path>` moves a secret out of layers and source files into `<project>/.secrets/restricted.config.yaml` (versioned map: cipher, tier, optional note, secured_at/secured_by) and splices a `⛔` sentinel into the source. The sentinel resolves only via the shadow file with the stricter reveal flag.

## `dc seal` — Inventory + In-Place (Un)Seal (`src/cmd/seal.rs`)

- **Scope**: project sources only — `.envrc*` heredoc blocks plus registered external (`dc load`) files. Store layer files are never rewritten (they are always encrypted at rest already).
- **Per-entry states**: `🔒 t<N>` (sealed, tier from token), `plain` (bare or `🔒`-marked plaintext), `⛔ shadow` (listed, never modified).
- **Modes**: interactive selector when stdin is a tty; `--headless` requires an action — `--encrypt KEY…` / `--encrypt-all-plain` / `--decrypt KEY…` / `--decrypt-all` (encrypt and decrypt groups are mutually exclusive at the parser). `--list [--json]` prints the inventory (key | file:line | state); `--dry-run` previews; `--yes` applies without prompting; `--file` restricts scope to one file.
- **Selectors**: `--match PAT` (repeatable, OR) keeps entries whose **leaf key name** — the last dot-segment only — matches a case-insensitive wildcard (`*` = any run incl. none, `?` = exactly one char, all else literal, anchored to the whole name). `--except PAT` (same syntax) always wins. Selectors apply in every mode and compose with `--dry-run`/`--json` (which report `selected` + `selected_keys`). Matching zero keys exits non-zero — never a silent success.
- **Rewrite guarantees**: reuses the `dc config set` splice machinery, so surrounding comments/formatting survive and untouched lines re-emit byte-identical. Encrypt collapses the value to one quoted line `🔒<bangs> <token>`; decrypt restores `🔒<bangs> <plaintext>` — byte-exact for values seal itself encrypted from `🔒`-marked originals; bare originals return as `🔒 <plaintext>`. Restore tier: original marker tier, else the token's own tier, else 0.
- **Known limitation**: multi-line block-scalar values collapse to one quoted line on rewrite.

## Seal Lifecycle

Typical flow: author plaintext under `🔒` markers in `.envrc.dc` → `dc seal` (interactive) or `dc seal --headless --encrypt --match '*password'` seals at rest → `dc yaml`/`dc load` ingest keeps tokens verbatim in `secrets.yaml` → consumers (`dc get`, SDKs, flatten/exports) carry the opaque token; only explicit audited reveals decrypt.
