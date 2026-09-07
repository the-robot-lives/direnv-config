# Project Layout — Summary

```
direnv-config/
├── src/                        # Rust CLI (dc)
│   ├── cmd/                    #   Subcommands
│   ├── envrc/                  #   .envrc* editing
│   ├── store/                  #   Store operations
│   ├── yaml/                   #   YAML utilities
│   ├── main.rs
│   └── *.rs                    #   Secrets, crypto, Infisical, kube, settings
├── bin/                        # dc-init, tabbing-on-step
├── lib/direnv-stdlib.sh        # direnv stdlib extension
├── shell/dc.zsh                # Deprecated stub → completions/_dc
├── completions/                # _dc (zsh), dc.bash (bash) — make install-completions
├── sdk/                        # Multi-language SDKs
│   ├── contract-tests/
│   ├── elixir/
│   ├── php/
│   ├── python/
│   ├── rust/
│   └── typescript/
├── demo/                       # Demo/test environments
├── docs/                       # Documentation (arch/, howto/, layout/; PROJ-ARCH/HOWTO/FAQ/LAYOUT)
├── .github/workflows/          # CI + SDK publishing
├── Cargo.toml
├── Makefile
├── CHANGELOG.md
├── merge-notes.md              # Branch-sweep provenance (sep-1 2026-09-01)
├── LICENSE
└── README.md
```
