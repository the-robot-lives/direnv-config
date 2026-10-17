# Project Layout — Summary

```
direnv-config/
├── src/                        # Rust CLI (dc) → layout/src.md
│   ├── cmd/                    #   One file per subcommand
│   ├── store/                  #   Store layout, locking, resolution, versioning
│   └── main.rs                 #   Entry point
├── bin/                        # dc-init (shell hook), tabbing-on-step (Zellij titles)
├── lib/direnv-stdlib.sh        # direnv stdlib extension (dc_yaml, dc_load, dc_export, …)
├── shell/dc.zsh                # Deprecated stub → completions/_dc
├── completions/                # _dc (zsh), dc.bash (bash) — make install-completions
├── sdk/                        # Multi-language SDKs → layout/sdk.md
│   ├── contract-tests/         #   Shared fixtures + expectations
│   ├── elixir/ php/ python/ rust/ typescript/
│   └── Makefile                #   Cross-SDK build/test/publish
├── demo/                       # Demo trees: root/, k8/, expected-state/
├── tests/                      # Rust integration tests (seal, emit, config set, …)
├── web/                        # Elixir Hologram landing site → layout/web.md
├── helm/dirconfig-site/        # Helm chart for the landing site
├── docs/                       # arch/, howto/, layout/; PROJ-* overviews, THREAT-MODEL, TEST-HEALTH
├── .github/workflows/          # CI (ci.yml) + SDK publishing (publish-sdks.yml)
├── Cargo.toml / Cargo.lock     # Rust manifest (binary: dc) + pinned deps
├── Makefile                    # compile, install, test, check, doctor, sdk-*
├── CHANGELOG.md                # Dated work log
├── merge-notes.md              # Branch-sweep provenance (sep-1, 2026-09-01)
├── LICENSE                     # MIT
├── README.md                   # Overview + CLI reference
├── CLAUDE.md                   # Agent guidance (AGENTS.md / AGENT.md siblings)
└── .gitignore                  # target/, .env, .envrc.local, .claude/worktrees/
```
