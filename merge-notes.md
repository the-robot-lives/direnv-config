# merge-notes — direnv-config (sep-1 branch sweep, 2026-09-01)

Utility repo (`the-robot-lives/direnv-config`; dual checkout `Portfolio/Utilities/source/direnv-config` + `utilities/...`, same remote — swept from the source checkout). sep-1 added alongside 2 pre-existing tags.

- Base: `origin/mono-repo-dev` @ 10ab6c4 (freshest tip by commit date; mono-repo-dev carries the real unmerged content vs a stale `main`).
- `develop` cut from mono-repo-dev 10ab6c4 and pushed; **mono-repo-dev retained as historical** — develop IS its continuation; do not open a PR for mono-repo-dev.
- `sep-1` tag created on 10ab6c4 (base tip).
- Local `main` left as-is / ff-synced to `origin/main` (was stale); main never pushed. Fast-forward `develop` → `main` when adopting.
- Branches pruned: none (repo held only `main` + `mono-repo-dev`; mono-repo-dev kept per sweep policy).

## Review/merge sequence
1. Review `develop` (== mono-repo-dev 10ab6c4) vs `main`; merge `develop` → `main` (FF) when validated. No other pending work; no open PRs.

## Open PRs
None.
