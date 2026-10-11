import Config

# Secret material for a static, cookie-less marketing site: there is no
# deploy-time secret for the helm chart to carry, so prefer an injected value
# when one is present and otherwise generate a strong one per boot. Sessions
# will NOT survive a restart — intentional for this site; do not "fix" that by
# committing a literal secret_key_base or signing salt.
secret_key_base =
  System.get_env("SECRET_KEY_BASE") ||
    (:crypto.strong_rand_bytes(48) |> Base.encode64() |> binary_part(0, 64))

# Plug options are evaluated at compile time, so the session salt is read at
# runtime by DirenvConfigWeb.Plugs.Session.init/1 from app env (see
# config.exs: :plug_init_mode, :runtime).
session_signing_salt =
  System.get_env("SESSION_SIGNING_SALT") ||
    (:crypto.strong_rand_bytes(24) |> Base.encode16(case: :lower) |> binary_part(0, 48))

config :direnv_config_web, DirenvConfigWeb.Endpoint,
  secret_key_base: secret_key_base,
  session_signing_salt: session_signing_salt,
  live_view: [signing_salt: session_signing_salt]

# Prod-only overrides for `mix release` containers. Only evaluated by the
# release's config provider — dev/test keep the settings in config.exs.
if config_env() == :prod do
  ip =
    System.get_env("BIND_IP", "0.0.0.0")
    |> String.split(".")
    |> Enum.map(&String.to_integer/1)
    |> List.to_tuple()

  port = System.get_env("PORT", "8160") |> String.to_integer()

  config :direnv_config_web, DirenvConfigWeb.Endpoint,
    url: [host: System.get_env("PHX_HOST", "dirconfig.therobotlives.com")],
    http: [ip: ip, port: port],
    server: true
end
