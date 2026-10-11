# syntax=docker/dockerfile:1.7
#
# direnv-config landing — server-rendered Phoenix/Hologram app (NOT a static site).
# Final stage MUST be named `AS production` (deploy tooling depends on it).
# Mirrors the run-claude web Dockerfile: hex/rebar/deps on BuildKit cache
# mounts; _build deliberately in the layer (Hologram bundler renames
# priv output across devices → EXDEV if _build were a cache mount).

ARG BASE_IMAGE="elixir:1.18.4-otp-28-slim"

FROM ${BASE_IMAGE} AS build

# Node 20+ required by the Hologram compiler (Elixir → JS bundles)
RUN apt-get update -y && \
    apt-get install -y build-essential git curl ca-certificates wget && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cross-arch emulation (QEMU / Rosetta) breaks BEAM's double-mapped RWX JIT
# pages and can corrupt compiler state during mix deps.compile.
ENV ERL_AFLAGS="+JMsingle true"

# Keep emulated builds below the scheduler/RSS cliff.
ENV ERL_FLAGS="+S 2:2"
ENV ELIXIR_ERL_OPTIONS="+S 2:2"

RUN mix local.hex --force && mix local.rebar --force

ENV MIX_ENV=prod

COPY mix.exs mix.lock ./
# deps.get is repeated in every deps-consuming RUN: gha layer-cache hits skip
# the earlier deps.get layer, and the gha exporter does NOT persist type=cache
# mounts, so a cold CI runner mounts an empty /app/deps and compile dies with
# "the dependency is not available". In-RUN deps.get is a no-op when the mount
# is warm and repopulates it when cold.
RUN --mount=type=cache,id=dcweb-mix-hex,target=/root/.hex,sharing=locked \
    --mount=type=cache,id=dcweb-mix-rebar,target=/root/.cache/rebar3,sharing=locked \
    --mount=type=cache,id=dcweb-mix-deps,target=/app/deps,sharing=locked \
    mix deps.get --only $MIX_ENV && \
    mkdir -p config

# Whole config dir: config.exs imports runtime.exs, which mix evaluates
# during deps.compile — so runtime.exs must exist before that step.
COPY config/ config/
RUN --mount=type=cache,id=dcweb-mix-hex,target=/root/.hex,sharing=locked \
    --mount=type=cache,id=dcweb-mix-rebar,target=/root/.cache/rebar3,sharing=locked \
    --mount=type=cache,id=dcweb-mix-deps,target=/app/deps,sharing=locked \
    mix deps.get --only $MIX_ENV && \
    MIX_OS_DEPS_COMPILE_PARTITION_COUNT=1 mix deps.compile

COPY lib lib
COPY app app
COPY priv priv

RUN --mount=type=cache,id=dcweb-mix-hex,target=/root/.hex,sharing=locked \
    --mount=type=cache,id=dcweb-mix-rebar,target=/root/.cache/rebar3,sharing=locked \
    --mount=type=cache,id=dcweb-mix-deps,target=/app/deps,sharing=locked \
    mix deps.get --only $MIX_ENV && \
    mix compile

RUN --mount=type=cache,id=dcweb-mix-hex,target=/root/.hex,sharing=locked \
    --mount=type=cache,id=dcweb-mix-rebar,target=/root/.cache/rebar3,sharing=locked \
    --mount=type=cache,id=dcweb-mix-deps,target=/app/deps,sharing=locked \
    mix deps.get --only $MIX_ENV && \
    mix release && cp -a /app/_build/${MIX_ENV}/rel/direnv_config_web /app/release && \
    find /app/release -name "*.js.map" -delete
# Hologram always emits source maps (--sourcemap is hardcoded in its esbuild
# invocation) and their sourcesContent embeds absolute build-environment paths
# (the page entry modules import deps by absolute path). Plug.Static serves the
# whole hologram/ dir, so strip the maps from the release tree: they are a
# debugging aid, not a runtime dependency. .dockerignore keeps locally built
# maps out of the context; this removes the ones the in-container build makes.

FROM ${BASE_IMAGE} AS production

WORKDIR /app
RUN chown nobody /app && \
    apt-get update -y && \
    apt-get install -y --no-install-recommends wget && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

# HOLOGRAM_START=1 is required or Hologram routes 404.
ENV MIX_ENV=prod \
    HOLOGRAM_START=1 \
    PHX_HOST=dirconfig.therobotlives.com

# Hologram.env/0 reads HOLOGRAM_ENV || MIX_ENV at RUNTIME and falls back to
# :dev when neither is set (true in a mix release) — which disables the
# Hologram router and 404s every page. Pin the env explicitly.
ENV HOLOGRAM_ENV=prod

COPY --from=build --chown=nobody:root /app/release ./

USER nobody

EXPOSE 8160

HEALTHCHECK --interval=15s --timeout=3s --start-period=15s --retries=3 \
    CMD wget -qO- http://localhost:8160/healthz || exit 1

CMD ["bin/direnv_config_web", "start"]
