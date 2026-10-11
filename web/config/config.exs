import Config

config :direnv_config_web, DirenvConfigWeb.Endpoint,
  url: [host: "localhost"],
  adapter: Bandit.PhoenixAdapter,
  render_errors: [formats: [html: DirenvConfigWeb.ErrorHTML], layout: false],
  pubsub_server: DirenvConfigWeb.PubSub,
  live_view: [signing_salt: "direnvConfigWeb"],
  secret_key_base: "dC8wQ3eR7tY1uI5oP9aS2dF4gH6jK8lZ0xC1vB3nM5qW7eR9tY2uI4oP6aS8dFgHk",
  http: [ip: {127, 0, 0, 1}, port: 8160],
  server: true

config :phoenix, :json_library, Jason

config :logger, level: :info

# Runtime overrides (PORT / BIND_IP / PHX_HOST) — prod only, see runtime.exs.
import_config "runtime.exs"

# Test overrides (no HTTP listener).
if config_env() == :test, do: import_config("test.exs")
