# src/ Layout

Rust source for the `dc` CLI binary.

```
src/
├── cmd/                    # Subcommand implementations
│   ├── bat.rs              #   dc bat — browse/search config entries, secrets masked
│   ├── bump.rs             #   dc bump — increment store version
│   ├── compare.rs          #   dc compare — compare local secret vs target URI (no-leak)
│   ├── config.rs           #   dc config get/set/setall — edit secrets in source .envrc* files
│   ├── decrypt.rs          #   dc decrypt — decrypt secret values
│   ├── encrypt.rs          #   dc encrypt — encrypt secret values
│   ├── env.rs              #   dc env — emit resolved environment exports
│   ├── gen.rs              #   dc gen — generate values (passwords etc.)
│   ├── gen_secrets.rs      #   Dice-marker (🎲) secret generation walker
│   ├── get.rs              #   dc get — read a config value
│   ├── infisical.rs        #   dc infisical — secrets-map-driven compare/get/set vs Infisical
│   ├── init.rs             #   dc init — initialize a store
│   ├── list.rs             #   dc list — list stores/configs
│   ├── mod.rs              #   Subcommand registry
│   ├── prune.rs            #   dc prune — remove stale store state
│   ├── purge.rs            #   dc purge — delete store state
│   ├── push.rs             #   dc push — push secret to infisical:// or kubernetes:// target
│   ├── secrets.rs          #   dc secrets — secret listing/operations
│   ├── set.rs              #   dc set — write a config value
│   ├── status.rs           #   dc status — store status report
│   ├── unset.rs            #   dc unset — remove a config value
│   └── yaml.rs             #   dc yaml — merge YAML from stdin into a named config
├── envrc/                  # Hand-authored .envrc* file editing
│   ├── locator.rs          #   Scanner for dc_yaml heredoc blocks in .envrc* files
│   └── mod.rs              #   Line-oriented .envrc* editing
├── store/                  # Store state operations
│   ├── layout.rs           #   On-disk store layout
│   ├── lock.rs             #   Advisory exclusive store lock (released on drop)
│   ├── meta.rs             #   Store metadata
│   ├── mod.rs              #   Store API
│   ├── resolve.rs          #   Layer resolution
│   └── version.rs          #   Version tracking
├── yaml/                   # YAML utilities
│   ├── flatten.rs          #   Flattening rules
│   ├── merge.rs            #   Deep merge
│   ├── mod.rs              #   YAML API
│   └── path.rs             #   Path expressions
├── audit.rs                # Append-only audit log for secret reveals
├── crypto.rs               # Reversible authenticated encryption for secret values
├── infisical.rs            # Native Infisical API client (universal-auth, secrets v3, folders v2)
├── kube.rs                 # Kubernetes Secret access via kubectl (read + create/patch)
├── main.rs                 # Entry point + clap CLI definition
├── secret.rs               # Secret marking scheme and split-and-encrypt walker
├── secretcmp.rs            # No-leak secret comparison
├── secretsmap.rs           # Parser + resolver for .infisical-secrets.yaml
├── settings.rs             # Global settings (~/.config/direnv-config/settings.yaml)
├── shadow.rs               # Shadow ("locked-down") secret store
└── target.rs               # Target URI grammar for dc compare / dc push
```
