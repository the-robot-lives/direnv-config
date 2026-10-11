defmodule DirenvConfigWeb.Controllers.Health do
  @moduledoc "Trivial liveness/readiness endpoint for k8s probes."

  use Phoenix.Controller, formats: [html: DirenvConfigWeb.ErrorHTML]

  def show(conn, _params), do: send_resp(conn, 200, "ok")
end
