# FAQ — Question Index

Companion to [PROJ-FAQ.md](PROJ-FAQ.md). Question headings only, for cheap relevance checks.

## Motivation
- Why would I use YAML files instead of plain `export VAR=value` in `.envrc`?
- Why store secrets in an encrypted `secrets.yaml` instead of just gitignoring a `.env` file?
- Why does a value need layering at all — why not one YAML file per config?

## Fit
- When is `dc` the wrong tool for a project?
- When should I use a monorepo's shared config instead of a per-project `dc` store?

## Comparison
- How is `dc` different from direnv's own layout/`.envrc` includes?
- How does this differ from just using Infisical or Vault directly?
- How does `dc`'s "prune" differ from "purge"?

## Capability
- Can two shells/scripts write to the store at the same time safely?
- Can a non-Rust app (Node, Python, Elixir, PHP) read `dc` config without shelling out?
- Can I get a value from a config without decrypting it, even to just inspect structure?

## Caveats
- What happens if the machine holding the encryption key is lost or wiped?
- What's the cost of the `precmd` shell hook — does it slow down every prompt?
- Does deep-merging config across layers ever silently lose data?
- Why does the tab title only update on the shell's next prompt instead of pushing instantly?
- Why do complex paths like `key[*].field` require `yq` installed instead of working out of the box?

## Trust
- Does anything ever leave my machine without me running a sync command explicitly?
- If I `dc get --reveal` a secret, is that logged anywhere I should worry about?
