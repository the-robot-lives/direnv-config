# How to: tag, read, and rotate secrets safely

**Goal:** keep sensitive values encrypted at rest, redacted by default, and audited whenever revealed — without hand-rolling encryption in `.envrc`.
**Prereqs:** a 32-byte AEAD key at `~/.config/direnv-config/settings.yaml` (`openssl rand -base64 32` → `key: "<value>"`).

### Tag a value as a secret

Prefix the YAML scalar with `🔒` (optionally `❗`×0-3 for sensitivity tier) in any `dc_yaml`/`dc yaml` heredoc — `dc` encrypts it and routes it to `secrets.yaml` regardless of `--layer`:

```bash
dc yaml cf <<'YAML'
account_id: abc123
api_token: "🔒❗❗ the-real-token"
YAML
```

### Read it back

```bash
dc get cf api_token              # 🔒 **redacted** — default, no key touched
dc get cf api_token --reveal     # decrypts to stdout — audited
```

### Generate a fresh random secret

```bash
dc gen 32 --raw                          # 32-char plaintext to stdout (pipe into a system, don't store raw)
dc gen 32 --hex                          # hex charset, encrypted 🔒:v1 token (default)
```

Or seed placeholders across a whole file and fill them in one pass:

```bash
# seed.yaml contains: password: "🎲 db_pass 🔒 hex 32"
dc gen-secrets --file seed.yaml --inline   # generates + encrypts in place
```

### Bulk-audit what's tagged, without exposing values

```bash
dc secrets                       # list secret key names per config
dc bat --all                     # every config, secrets shown as 🔒 **redacted**
dc bat cf --filter-key 'token'   # narrow to matching keys, still redacted
```

### Edit a secret directly in `.envrc` source

```bash
dc config get cf api_token                        # shows file:line + redacted preview
dc config set cf api_token --value "$NEW_TOKEN"    # encrypts + splices into the heredoc in place
```

**Verify:** `dc get cf api_token --reveal` prints the new value; `git diff .envrc` shows only the `🔒:v1:...` token changed, never plaintext.

**Gotchas:**
- Rotating the AEAD key in `settings.yaml` orphans every existing `🔒:v1` token — `dc` fails loudly rather than silently re-encrypting. Re-tag and re-encrypt affected secrets after a key rotation.
- `dc get --reveal` still won't show a value that's been moved to the shadow store (`⛔` sentinel) — that requires `dc get ... --reveal-restricted` (see `dc config secure` in the README's Secret Management section).
- Every `--reveal`/`--clippy`/`--reveal-restricted` call appends a line to the audit log (mode `0600`) — don't script bulk reveals in a loop without expecting a matching audit trail.
