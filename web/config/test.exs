import Config

# Test overrides — no HTTP listener; requests go through Phoenix.ConnTest.
config :direnv_config_web, DirenvConfigWeb.Endpoint,
  server: false,
  http: [ip: {127, 0, 0, 1}, port: 8161]
