# Project Layout

`direnv-config` — YAML-backed configuration + secrets layer for [direnv](https://direnv.net/). Rust CLI (`dc`) with multi-language SDK clients, secret encryption, and Infisical/Kubernetes integration.

```
direnv-config/
├── src/                        # Rust CLI source → [layout/src.md](layout/src.md)
│   ├── cmd/                    #   Subcommands (get, set, env, bat, compare, config, encrypt/decrypt, gen, infisical, push, secrets, …)
│   ├── envrc/                  #   Hand-authored .envrc* editing (heredoc locator)
│   ├── store/                  #   Store operations (layout, lock, meta, resolve, version)
│   ├── yaml/                   #   YAML utilities (flatten, merge, path expressions)
│   ├── main.rs                 #   Entry point
│   └── *.rs                    #   Secrets/crypto/audit, Infisical client, kubectl access, settings
├── bin/
│   ├── dc-init                 #   Shell initializer (zsh hook, IPC watcher)
│   └── tabbing-on-step         #   Zellij pane-title helper (step name + emoji)
├── lib/
│   └── direnv-stdlib.sh        #   direnv stdlib extension (dc_yaml, dc_export, dc_set, etc.)
├── shell/
│   └── dc.zsh                  #   Deprecated stub — completion moved to completions/_dc
├── completions/
│   ├── _dc                     #   Zsh completion (install: make install-completions)
│   └── dc.bash                 #   Bash completion (bash-completion v2 auto-load)
├── sdk/                        # Client libraries → [layout/sdk.md](layout/sdk.md)
│   ├── contract-tests/         #   Shared test fixtures and expectations
│   ├── elixir/                 #   Elixir SDK (:direnv_config)
│   ├── php/                    #   PHP SDK (noizu/direnv-config)
│   ├── python/                 #   Python SDK (noizu-direnv-config)
│   ├── rust/                   #   Rust SDK (direnv-config, read + write)
│   ├── typescript/             #   TypeScript SDK (@noizu/direnv-config)
│   ├── Makefile                #   Cross-SDK build/test/publish
│   └── README.md               #   SDK overview and quick-start
├── demo/                       # Demo environments for testing
│   ├── expected-state/         #   Expected resolved YAML per demo scenario
│   ├── k8/                     #   Simulated k8 infra tree with .envrc files
│   ├── root/                   #   Simulated project root with nested .envrc files
│   └── README.md               #   Demo usage guide
├── docs/                       # Documentation
│   ├── arch/                   #   Architecture notes (flatten-rules, layer-resolution, parent-chain)
│   ├── layout/                 #   Detailed layout breakdowns (src.md, sdk.md)
│   └── howto/                  #   Step-by-step guides (first-hour, manage-secrets, migrate-envrc, …)
│   ├── PROJ-ARCH.md            #   Architecture overview (+ .summary.md)
│   ├── PROJ-HOWTO.md           #   Task-oriented howto index (+ .summary.md) → [howto/](howto/)
│   ├── PROJ-FAQ.md             #   FAQ (+ .summary.md)
│   ├── PROJ-LAYOUT.md          #   This file
│   └── PROJ-LAYOUT.summary.md  #   Quick-reference tree
├── .github/workflows/          # CI (ci.yml) and SDK publishing (publish-sdks.yml)
├── Cargo.toml                  # Rust package manifest (binary: dc)
├── Makefile                    # Build, install, test, check, doctor, install-completions, clean
├── CHANGELOG.md                # Release history
├── merge-notes.md              # Branch-sweep provenance note (sep-1 2026-09-01)
├── LICENSE                     # MIT
├── README.md                   # Project overview and usage
└── .gitignore                  # Excludes: target/, .env, .envrc.local
```

## Key Files Requiring Setup

| File | Action |
|------|--------|
| `Makefile` | `make install` — builds binary, installs direnv stdlib, adds shell hook |
| `lib/direnv-stdlib.sh` | Symlinked to `~/.config/direnv/lib/dc.sh` by `make install` |
| `bin/dc-init` | Installed to `~/.local/bin/dc-init`; sourced in `.zshrc` |

## Installed Locations

| Component | Path |
|-----------|------|
| CLI binary | `~/.local/bin/dc` |
| Shell initializer | `~/.local/bin/dc-init` |
| direnv stdlib | `~/.config/direnv/lib/dc.sh` → symlink to `lib/direnv-stdlib.sh` |
| Shell completions | bash: `~/.local/share/bash-completion/completions/dc`; zsh: `~/.local/share/zsh/site-functions/_dc` (`make install-completions`) |
| Runtime state | `~/.local/state/direnv-config/{path-hash}/` |
| Global settings | `~/.config/direnv-config/settings.yaml` |
