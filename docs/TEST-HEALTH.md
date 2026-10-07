# Test Health — direnv-config
_Last measured: 2026-10-07 · branch develop@4faa4a1_

Rust CLI (`dc`) plus five SDKs (Rust, TypeScript, Python, Elixir, PHP). No docker image; the CLI
is installed locally via `make install`, SDKs publish on `sdk-v*` tags (`publish-sdks.yml`).

| Metric | Before | After |
|---|---|---|
| CI PR wall-clock — critical path (warm / cold) | ~57s cold (Rust CLI); **never green** — clippy `-D warnings` failed on every run since 2026-05 | measure post-merge (expect ~40s warm: tests + clippy split, rust-cache) |
| Main release build (warm / cold) | n.a. — no image; SDK publish on tags | n.a. |
| CI acceptance test job (warm / cold) | Rust CLI `cargo test` 28–30s cold (compile-dominated) | rust-cache; see report |
| Local full-suite runtime (uptime load) | CLI `cargo test` 41s incl. compile, tests 0.01s (load 27.7) | unchanged |
| Docker build (warm / cold) | n.a. | n.a. |
| Tests in acceptance / slow tier | CLI 148 · Rust SDK 71+1 · Python 114 · Elixir 89 · TS/PHP suites (7 files each) / 0 slow | same / 0 |
| Async modules / total | n.a. (Rust tests run threaded by default) | n.a. |
| Coverage — acceptance pass | not measured | CLI 55.65% · Rust SDK 84.45% · Python 66% · Elixir 52.41% (lines) |
| Coverage — full pass | = acceptance | = acceptance |
| Coverage gate | — | CLI 50% · Rust SDK 79% · Python 61% · Elixir 47% |

CI triggers also changed: previously only `pull_request`/`push` on **main**, so feature PRs into
`develop` ran no CI at all. Now `develop` is covered for both events (no publish/bump steps live in
`ci.yml`, so nothing release-side moves).

## Caching status
- GitHub Actions: cargo (Swatinem/rust-cache) ✅ · mix deps+_build ✅ · npm ✅ · pip ✅ · composer ❌ (11s job, not worth it) · PLT n.a. · develop-ref seeding ✅
- Docker: n.a. (no image)

## Slow tests (tier: nightly)
None — every suite finishes in < 1s of test time; CI time is toolchain setup + compile.

## Test debt
| Item | Kind | Notes |
|---|---|---|
| TypeScript SDK coverage | coverage gap | no `@vitest/coverage-v8` devDependency; add it + `coverage.thresholds.lines` in a follow-up |
| PHP SDK coverage | coverage gap | needs pcov/xdebug via `setup-php coverage:`; not wired |
| CLI coverage 55% | coverage gap | `cmd/*` handlers (interactive / kube / infisical paths) largely untested |
| `KeySource.path`, `Settings.key` | dead fields | `#[allow(dead_code)]` added to unblock clippy; decide whether to surface or remove |
| Python publish ran no tests | release gate | `publish-sdks.yml` python job now runs `pytest` before `build`; other SDK publish jobs already tested |

## Nightly
Not needed — no slow tier, no docker cache to warm.
