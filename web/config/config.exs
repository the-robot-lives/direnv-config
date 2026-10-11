import Config

config :direnv_config_web, DirenvConfigWeb.Endpoint,
  url: [host: "localhost"],
  adapter: Bandit.PhoenixAdapter,
  render_errors: [formats: [html: DirenvConfigWeb.ErrorHTML], layout: false],
  pubsub_server: DirenvConfigWeb.PubSub,
  http: [ip: {127, 0, 0, 1}, port: 8160],
  server: true

# Secret material (secret_key_base + signing salts) is injected or generated
# per boot in runtime.exs — nothing secret is committed here.
#
# Plug options in endpoint.ex are evaluated at compile time, so plugs that
# need boot-time values read them inside init/1; runtime init mode makes that
# init run when the endpoint serves instead of when it compiles.
config :phoenix, :plug_init_mode, :runtime

config :phoenix, :json_library, Jason

config :logger, level: :info

# Runtime config: secrets for every env, prod overrides (PORT / BIND_IP /
# PHX_HOST) — see runtime.exs.
import_config "runtime.exs"

# Test overrides (no HTTP listener).
if config_env() == :test, do: import_config("test.exs")
