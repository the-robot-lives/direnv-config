defmodule DirenvConfigWeb.Endpoint do
  use Phoenix.Endpoint, otp_app: :direnv_config_web

  # Hologram's client runtime is served from the "hologram" path;
  # our own assets from "assets".
  plug Plug.Static,
    at: "/",
    from: :direnv_config_web,
    only: ["assets", "hologram"],
    gzip: false

  plug Plug.Parsers,
    parsers: [:urlencoded, :multipart, :json],
    pass: ["*/*"],
    json_decoder: Jason

  plug Plug.Session,
    store: :cookie,
    key: "_direnv_config_web_key",
    signing_salt: "direnvConfigWeb"

  # Hologram's router must run BEFORE the Phoenix router.
  plug Hologram.Router
  plug DirenvConfigWeb.Router
end
