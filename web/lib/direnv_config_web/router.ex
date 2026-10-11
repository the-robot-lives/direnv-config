defmodule DirenvConfigWeb.Router do
  use Phoenix.Router

  get "/healthz", DirenvConfigWeb.Controllers.Health, :show
end
