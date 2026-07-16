# How to: install direnv-config and verify it works

**Goal:** get `dc` on your PATH, wired into direnv, and confirm a live `.envrc` exports env vars from YAML.
**Prereqs:** [direnv](https://direnv.net/) installed and hooked into your shell; Rust toolchain if building from source (`rustup.rs`).

1. Build the binary:
   ```bash
   cd utilities/shell/direnv-config
   make compile
   ```
2. Install everything in one step (binary → `~/.local/bin/dc`, direnv stdlib symlink, `dc-init` shell hook in `~/.zshrc`):
   ```bash
   make install
   source ~/.zshrc   # or open a new shell
   ```
3. Confirm each piece landed:
   ```bash
   make check
   ```
   Expect four `✓` lines: `dc` binary, direnv stdlib symlink, shell hook, `direnv` itself.
4. Create a test project and a minimal `.envrc`:
   ```bash
   mkdir -p /tmp/dc-demo && cd /tmp/dc-demo
   cat > .envrc <<'EOF'
   dc_yaml tab <<'YAML'
   theme: kanagawa
   status: idle
   YAML
   dc_export
   EOF
   direnv allow
   ```
5. Enter the directory (or `direnv reload`) and check the var landed:
   ```bash
   cd /tmp/dc-demo && echo "$TAB_THEME"
   ```

**Verify:**
```bash
dc status
# Source, Store path, Environment, Version, Configs: tab
```

**Gotchas:**
- `make check` shows `✗ dc binary: not found` → you skipped `make compile`, or `~/.local/bin` isn't on `PATH` yet; re-`source ~/.zshrc`.
- `.envrc` vars don't appear → you forgot `direnv allow` (direnv blocks unapproved `.envrc` files by default), or the direnv stdlib symlink is missing (`make install-direnv-lib`).
- Editing `.envrc` and re-entering the directory doesn't pick up changes → run `direnv allow` again; direnv re-blocks on any file diff.
- No live IPC (tab title doesn't update from a background script) → the shell hook step didn't run; check `grep dc-init ~/.zshrc` and re-open the shell.
