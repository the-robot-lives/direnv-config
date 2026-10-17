# Project Layout

`direnv-config` — YAML-backed configuration + secrets layer for [direnv](https://direnv.net/). Rust CLI (`dc`) with multi-language SDK clients, secret encryption, and Infisical/Kubernetes integration.

```
direnv-config/
├── src/                        # Rust CLI source (binary: dc) → [layout/src.md](layout/src.md)
│   ├── cmd/                    #   One file per subcommand (yaml, get/set, seal, emit, load, infisical, keys, …)
│   ├── store/                  #   Store state: on-disk layout, locking, layer resolution, versioning
│   └── main.rs                 #   Entry point + clap CLI definition
├── bin/
│   ├── dc-init                 #   Shell initializer — emits the zsh/bash IPC hook (eval "$(dc-init zsh)")
│   └── tabbing-on-step         #   Zellij pane-title helper (step name + emoji)
├── lib/
│   └── direnv-stdlib.sh        #   direnv stdlib extension (dc_yaml, dc_load, dc_export, dc_set, …)
├── shell/
│   └── dc.zsh                  #   Deprecated stub — completion moved to completions/_dc
├── completions/
│   ├── _dc                     #   Zsh completion (install: make install-completions)
│   └── dc.bash                 #   Bash completion (bash-completion v2 auto-load)
├── sdk/                        # Client libraries (elixir, php, python, rust, typescript) → [layout/sdk.md](layout/sdk.md)
│   ├── contract-tests/         #   Shared fixtures + expectations all SDKs validate against
│   ├── elixir/                 #   Elixir SDK (:direnv_config)
│   └── Makefile                #   Cross-SDK build/test/publish orchestrator
├── demo/                       # Runnable demo tree exercising every dc operation
│   ├── root/                   #   Simulated project root with nested .envrc files + deploy script
│   ├── k8/                     #   Simulated infra tree (.envrc in bin/, docker/telemetry/, terraform/)
│   ├── expected-state/         #   Expected resolved YAML per demo scenario
│   └── README.md               #   Demo walkthrough
├── tests/                      # Rust integration tests (seal, emit, config set --parents, provenance routing, …)
├── web/                        # Elixir Hologram landing site (dirconfig.therrobotlives.com) → [layout/web.md](layout/web.md)
│   ├── app/                    #   Hologram pages/components
│   ├── lib/direnv_config_web/  #   Phoenix server modules
│   └── Dockerfile              #   Multi-stage container build (final stage: production)
├── helm/
│   └── dirconfig-site/         # Helm chart deploying the web/ site (deployment, service, ingress)
├── docs/                       # Documentation
│   ├── arch/                   #   Internals: flatten rules, layer resolution, parent chain
│   ├── howto/                  #   Task guides (first-hour, manage-secrets, migrate-envrc, sync-secrets-remote, use-sdk-from-app-code)
│   ├── layout/                 #   Per-directory breakdowns (src.md, sdk.md, web.md)
│   ├── PROJ-ARCH/-SCHEMA/-HOWTO/-FAQ .md  # Overviews, each with a .summary.md digest
│   ├── THREAT-MODEL.md         #   Security model (+ .summary.md)
│   ├── TEST-HEALTH.md          #   Test-suite health baseline
│   └── PROJ-LAYOUT.md          #   This file (+ .summary.md quick-reference tree)
├── .github/workflows/          # CI (ci.yml) + SDK publishing on sdk-v* tags (publish-sdks.yml)
├── Cargo.toml                  # Rust package manifest (binary: dc)
├── Cargo.lock                  # Pinned Rust dependency versions
├── Makefile                    # compile, test, install/uninstall, check, doctor, completions, sdk-* targets
├── CHANGELOG.md                # Dated work log, newest first
├── merge-notes.md              # Branch-sweep provenance note (sep-1, 2026-09-01)
├── LICENSE                     # MIT
├── README.md                   # Project overview, CLI reference, how-it-works
├── CLAUDE.md                   # Agent guidance (Claude Code)
├── AGENTS.md / AGENT.md        # Harness-shaped sibling rules (Codex/Grok/Cursor) + mirror
└── .gitignore                  # Excludes: target/, .env, .envrc.local, .claude/worktrees/
```

## Installed Locations

| Component | Path |
|-----------|------|
| CLI binary | `~/.local/bin/dc` |
| Shell initializer | `~/.local/bin/dc-init` (+ `tabbing-on-step`) |
| direnv stdlib | `~/.config/direnv/lib/dc.sh` (copy of `lib/direnv-stdlib.sh`; re-run `make install-direnv-lib` after edits) |
| Shell completions | bash: `~/.local/share/bash-completion/completions/dc`; zsh: `~/.local/share/zsh/site-functions/_dc` (`make install-completions`) |
| Runtime state | `~/.local/state/direnv-config/{path-hash}/` |
| Global settings | `~/.config/direnv-config/settings.yaml` |

## Key Files Requiring Setup

| File | Action |
|------|--------|
| `Makefile` | `make install` — builds `dc`, installs direnv stdlib + shell hook + completions |
| `lib/direnv-stdlib.sh` | Copied to `~/.config/direnv/lib/dc.sh` by `make install`; edits need `make install-direnv-lib` to take effect |
| `bin/dc-init` | Installed to `~/.local/bin/dc-init`; hook line added to `~/.zshrc` by `make install` |
| `completions/` | `make install-completions` (also run by `make install`) |
