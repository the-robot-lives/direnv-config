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
├── shell/dc.zsh                # Zsh completions
├── sdk/                        # Multi-language SDKs
│   ├── contract-tests/
│   ├── elixir/
│   ├── php/
│   ├── python/
│   ├── rust/
│   └── typescript/
├── demo/                       # Demo/test environments
├── docs/                       # Documentation (arch/, layout/)
├── .github/workflows/          # CI + SDK publishing
├── Cargo.toml
├── Makefile
├── CHANGELOG.md
├── LICENSE
└── README.md
```
