defmodule DirenvConfigWeb.Plugs.Session do
  @moduledoc """
  `Plug.Session` with a boot-time signing salt.

  The salt lives in app config (`config/runtime.exs`): the
  `SESSION_SIGNING_SALT` env var when set, otherwise a strong random value
  generated per boot. With `:plug_init_mode, :runtime` (config.exs), `init/1`
  runs when the endpoint serves rather than when it compiles, so the salt is
  never baked into a compiled artifact and no literal salt is committed.
  """

  @behaviour Plug

  @impl true
  def init(_opts) do
    signing_salt =
      :direnv_config_web
      |> Application.fetch_env!(DirenvConfigWeb.Endpoint)
      |> Keyword.fetch!(:session_signing_salt)

    Plug.Session.init(
      store: :cookie,
      key: "_direnv_config_web_key",
      signing_salt: signing_salt
    )
  end

  @impl true
  def call(conn, opts), do: Plug.Session.call(conn, opts)
end
