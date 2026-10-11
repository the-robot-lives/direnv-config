import Config

# Runtime overrides for `mix release` containers. Only evaluated by the
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
